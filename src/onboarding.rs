//! First-run presentation and profile creation stay in the client.
use crate::i18n::t;
use crate::{
    display::{ACCENT as GREEN, MUTED},
    terminal::{safe, text},
    ui::Pictures,
};
use anyhow::{bail, ensure, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};
use serde_json::{json, Value};
use shum_core::card::Card;
use shum_store::{
    profiles::{Profile, Profiles},
    vault::{KeyMode, ProfileKeys},
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

mod animation;
pub use animation::CreationAnimation;

/// Like iOS registration, choose the first portrait from the profile's public
/// signing key. The selected seed is explicit in the signed transport card;
/// Core's legacy default for cards without a selected seed remains unchanged.
pub fn initial_seed(keys: &ProfileKeys) -> u64 {
    shum_core::crypto::avatar_seed(&keys.signing.ed_public())
}

fn security_steps() -> [&'static str; 4] {
    [
        t("Криптографические ключи"),
        t("Шифрование"),
        t("Защищённое хранилище"),
        t("Проверка"),
    ]
}

/// Prepare and check security before asking for a name. Temporary encrypted
/// storage is removed before returning; secrets stay in memory until the user
/// confirms the profile. No provisional profile is published in the registry.
pub fn prepare(root: &Path, mut progress: impl FnMut(usize)) -> Result<ProfileKeys> {
    progress(0);
    let keys = ProfileKeys::generate()?;
    progress(1);
    let mut nonce = [0; 12];
    getrandom::fill(&mut nonce)?;
    let challenge = b"shum.registration.check.v1";
    let encrypted = shum_store::codec::seal(keys.storage.expose(), &nonce, challenge, b"")?;
    ensure!(
        shum_store::codec::open(keys.storage.expose(), &encrypted, b"")?.as_slice() == challenge,
        "{}",
        t("Не прошла проверка шифрования")
    );
    progress(2);
    Profiles::new(root)?;
    let temporary = tempfile::Builder::new()
        .prefix(".registration-")
        .tempdir_in(root)?;
    let database = temporary.path().join("check.sqlite");
    let card = Card::create(
        &keys.noise,
        &keys.signing,
        &keys.nostr,
        t("Подготовка").into(),
        "",
        Some(0),
        1,
    )?;
    let mut store = shum_store::Store::open(&database, &keys.owner_id(), keys.storage_key())?;
    store.transaction(|state| {
        state["ownProfileCard"] = serde_json::to_value(&card)?;
        Ok(())
    })?;
    store.checkpoint()?;
    drop(store);
    progress(3);
    let reopened = shum_store::Store::open(&database, &keys.owner_id(), keys.storage_key())?;
    let stored: Card = serde_json::from_value(reopened.state()["ownProfileCard"].clone())?;
    stored.validate()?;
    ensure!(
        stored == card,
        "{}",
        t("Не прошла проверка защищённого хранилища")
    );
    ensure!(
        shum_core::crypto::verify_ed(
            &keys.signing.ed_public(),
            &keys.signing.sign(challenge),
            challenge
        ),
        "{}",
        t("Не прошла проверка ключа профиля")
    );
    drop(reopened);
    temporary.close()?;
    progress(4);
    Ok(keys)
}

#[derive(Clone)]
pub struct Settings {
    pub bluetooth: bool,
    pub relays: Vec<String>,
    pub push_url: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            bluetooth: true,
            relays: shum_transport_nostr::DEFAULT_RELAYS
                .iter()
                .map(|s| (*s).into())
                .collect(),
            push_url: Some("https://d5d5lr0h6812sbjiiqoa.ccx97b51.apigw.yandexcloud.net".into()),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        shum_transport_nostr::RelayPool::validate_urls(&self.relays)?;
        if let Some(url) = &self.push_url {
            shum_transport_nostr::push::PushClient::new(url)?;
        }
        Ok(())
    }
}
pub fn validate_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= 64
            && name.trim() == name
            && !name.chars().any(char::is_control),
        "{}",
        t("Имя: 1–64 байта UTF-8, без пробелов по краям (до 32 русских букв)")
    );
    Ok(())
}
pub struct Created {
    pub profile: Profile,
    pub card: Card,
}
pub fn create(
    root: &Path,
    name: &str,
    seed: Option<u64>,
    mode: KeyMode,
    settings: &Settings,
    keys: ProfileKeys,
) -> Result<Created> {
    validate_name(name)?;
    settings.validate()?;
    let card = Card::create(
        &keys.noise,
        &keys.signing,
        &keys.nostr,
        name.into(),
        "",
        Some(seed.unwrap_or_else(|| initial_seed(&keys))),
        1,
    )?;
    card.validate()?;
    let profiles = Profiles::new(root)?;
    let p = profiles.create_with_keys(name, mode, keys)?;
    let save = (|| -> Result<()> {
        let mut open = profiles.open(Some(&p.id))?;
        open.store.transaction(|s| {
            s["ownProfileCard"] = serde_json::to_value(&card)?;
            s["cliSettings"] = json!({"relays":settings.relays,"pushURL":settings.push_url,"bluetooth":settings.bluetooth});
            Ok(())
        })?;
        drop(open);
        // Read back encrypted storage and verify the signed public card before reporting success.
        let reopened = profiles.open(Some(&p.id))?;
        let stored: Card =
            serde_json::from_value(reopened.store.state()["ownProfileCard"].clone())?;
        stored.validate()?;
        ensure!(
            stored == card && stored.id() == reopened.keys.owner_id(),
            "{}",
            t("Не прошла проверка сохранённого профиля")
        );
        drop(reopened);
        profiles.select(&p.id)?;
        Ok(())
    })();
    if let Err(error) = save {
        if let Err(cleanup) = profiles.delete(&p.id) {
            bail!(
                "{}",
                crate::i18n::format(
                    "{error}; не удалось удалить незавершённый профиль {}: {cleanup}",
                    &[
                        ("error", format!("{}", error)),
                        ("0", p.id.to_string()),
                        ("cleanup", format!("{}", cleanup))
                    ]
                )
            );
        }
        return Err(error);
    }
    Ok(Created { profile: p, card })
}
#[derive(Clone, Copy, PartialEq)]
pub enum Step {
    Name,
    Avatar,
    Saving,
    Done,
}
pub struct Wizard {
    pub step: Step,
    pub name: String,
    pub seed: u64,
    pub error: String,
    pub file_keys: bool,
}

pub fn draw_preparation(
    frame: &mut Frame<'_>,
    completed: usize,
    pictures: &mut Pictures,
    ascii: bool,
) {
    draw_animated_preparation(
        frame,
        &CreationAnimation::settled(completed.min(4)),
        pictures,
        ascii,
    );
}

/// Render a frame of the preparation ceremony without publishing an identity.
pub fn draw_animated_preparation(
    frame: &mut Frame<'_>,
    animation: &CreationAnimation,
    pictures: &mut Pictures,
    ascii: bool,
) {
    draw_security(frame, animation, None, "", ascii);
    pictures.colors.apply(frame.buffer_mut(), ascii);
}

fn draw_security(
    frame: &mut Frame<'_>,
    animation: &CreationAnimation,
    name: Option<&str>,
    error: &str,
    ascii: bool,
) {
    let completed = animation.completed();
    let area = frame.area();
    let color = |c| if ascii { Color::Reset } else { c };
    let green = Style::default().fg(color(GREEN));
    let muted = Style::default().fg(color(MUTED));
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(color(Color::Rgb(10, 13, 11)))),
        area,
    );
    if area.width < 35 || area.height < 18 {
        frame.render_widget(
            Paragraph::new(t("Увеличьте окно до 35×18. Esc отменить")),
            area,
        );
        return;
    }
    let x = area.x + 2;
    let width = area.width.saturating_sub(4);
    let row = |y| Rect::new(x, area.y + y, width, 1);
    frame.render_widget(
        Paragraph::new(t("Создаём вашу защиту")).style(green),
        row(1),
    );
    frame.render_widget(
        Paragraph::new(t(
            "Ключи создаются и сохраняются только на этом устройстве.",
        ))
        .style(muted)
        .wrap(Wrap { trim: false }),
        Rect::new(x, area.y + 3, width, 2),
    );
    let tall = area.height >= 32 && area.width >= 40 && !ascii;
    if tall {
        animation::draw_key(frame, x + 2, area.y + 6, 13, animation);
    }
    let y = if tall { 20 } else { 6 };
    for (index, title) in security_steps().iter().enumerate() {
        let (mark, style) = if index < completed {
            (if ascii { "+" } else { "✓" }, green)
        } else if index == completed {
            ("", green)
        } else {
            ("·", muted)
        };
        let mark = if index == completed {
            animation.spinner().to_string()
        } else {
            mark.into()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{mark} "), style),
                Span::styled(
                    *title,
                    if index <= completed {
                        Style::default()
                    } else {
                        muted
                    },
                ),
            ])),
            row(y + index as u16),
        );
    }
    let bar_width = 32.min(width.saturating_sub(6));
    let percent = animation.percent();
    let filled = bar_width * percent / 100;
    frame.render_widget(
        Paragraph::new(format!(
            "{}{} {}%",
            "=".repeat(usize::from(filled)),
            "·".repeat(usize::from(bar_width - filled)),
            percent
        ))
        .style(green),
        row(y + 4),
    );
    if !ascii {
        for dx in 0..filled {
            frame.buffer_mut()[(x + dx, area.y + y + 4)]
                .set_symbol(" ")
                .set_bg(color(GREEN));
        }
    }
    if completed == 4 {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(t("Ключ профиля создан. "), green),
                Span::styled(t("Закрытые ключи не покидают устройство."), muted),
            ]))
            .wrap(Wrap { trim: false }),
            Rect::new(x, area.y + y + 6, width, 2),
        );
    }
    if let Some(name) = name {
        let prefix = if width >= 60 {
            t("? Имя (до 64 байт UTF-8): ")
        } else {
            t("? Имя: ")
        };
        let prefix_width = unicode_width::UnicodeWidthStr::width(prefix) as u16;
        frame.render_widget(
            Paragraph::new(format!("{prefix}{}", safe(name))).style(green),
            row(y + 8),
        );
        frame.set_cursor_position((
            x + (prefix_width + unicode_width::UnicodeWidthStr::width(name) as u16)
                .min(width.saturating_sub(1)),
            area.y + y + 8,
        ));
    }
    if !error.is_empty() {
        frame.render_widget(
            Paragraph::new(safe(error)).style(Style::default().fg(color(Color::Yellow))),
            row(area.height - 3),
        );
    }
    frame.render_widget(
        Paragraph::new(if name.is_some() {
            t("Enter далее · Esc отменить · Ctrl+C выход")
        } else {
            t("Esc отменить · Ctrl+C выход")
        })
        .style(muted),
        row(area.height - 1),
    );
}
pub fn draw(frame: &mut Frame<'_>, wizard: &Wizard, pictures: &mut Pictures, ascii: bool) {
    draw_content(frame, wizard, pictures, ascii);
    pictures.colors.apply(frame.buffer_mut(), ascii);
}
fn draw_content(frame: &mut Frame<'_>, wizard: &Wizard, pictures: &mut Pictures, ascii: bool) {
    if wizard.step == Step::Name {
        draw_security(
            frame,
            &CreationAnimation::settled(4),
            Some(&wizard.name),
            &wizard.error,
            ascii,
        );
        return;
    }
    let area = frame.area();
    let color = |c| if ascii { Color::Reset } else { c };
    let muted = Style::default().fg(color(MUTED));
    let green = Style::default().fg(color(GREEN));
    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(color(Color::Rgb(10, 13, 11)))),
        area,
    );
    let x = area.x + 2.min(area.width);
    let width = area.width.saturating_sub(4);
    let row =
        |y: u16, h: u16| Rect::new(x, area.y + y, width, h.min(area.height.saturating_sub(y)));
    if area.width < 35 || area.height < 18 {
        frame.render_widget(
            Paragraph::new(t("Увеличьте окно до 35×18. Esc отменить")),
            area,
        );
        return;
    }
    frame.render_widget(
        Paragraph::new(t("Shum · Новый профиль")).style(green),
        row(1, 1),
    );
    frame.render_widget(
        Paragraph::new(t(
            "Ключи создаются и сохраняются только на этом устройстве.",
        ))
        .style(muted)
        .wrap(Wrap { trim: false }),
        row(3, 2),
    );
    {
        frame.render_widget(
            Paragraph::new(crate::i18n::format(
                "Имя: {}",
                &[("0", safe(&wizard.name).to_string())],
            )),
            row(6, 1),
        );
        frame.render_widget(
            Paragraph::new(t("Аватар · пиксельный, его видят все")).style(muted),
            row(8, 1),
        );
        let text_avatar = pictures.cell_avatars();
        let tall = if text_avatar {
            area.height >= 28 && area.width >= 60
        } else {
            area.height >= 24
        };
        let avatar_height = if tall && text_avatar {
            crate::avatar::CELL_SIDE
        } else if tall {
            9
        } else {
            3
        };
        let avatar_width = avatar_height * 2;
        if !ascii {
            pictures.draw(
                frame,
                wizard.seed,
                Rect::new(x + 2, area.y + 10, avatar_width, avatar_height),
            );
        }
        let dx = if ascii || (text_avatar && !tall) {
            0
        } else if tall {
            avatar_width + 6
        } else {
            10
        };
        let options = match wizard.step {
            Step::Avatar => vec![
                Line::from(Span::styled(&wizard.name, green)),
                Line::from(""),
                Line::from(t("[r] другой вариант")),
                Line::from(t("[Enter] оставить этот")),
                Line::from(t("[Esc] изменить имя")),
            ],
            Step::Saving => vec![
                Line::from(Span::styled(t("Сохраняем профиль…"), green)),
                Line::from(t("Шифруем базу и проверяем ключи.")),
            ],
            Step::Done => vec![
                Line::from(Span::styled(t("✓ Профиль готов"), green)),
                Line::from(t("✓ Ключи и шифрование")),
                Line::from(if wizard.file_keys {
                    t("✓ Файл ключей 0600")
                } else {
                    t("✓ Системное хранилище ключей")
                }),
                Line::from(t("✓ Проверка чтения и подписей")),
                Line::from(t("[Enter] продолжить")),
            ],
            Step::Name => unreachable!(),
        };
        frame.render_widget(
            Paragraph::new(options).wrap(Wrap { trim: false }),
            Rect::new(
                x + dx,
                area.y + 10,
                width.saturating_sub(dx),
                6.min(area.height.saturating_sub(12)),
            ),
        );
        if tall && wizard.step == Step::Avatar {
            frame.render_widget(
                Paragraph::new(t("Фото-аватары пока недоступны.")).style(muted),
                row(if text_avatar { 24 } else { 21 }, 1),
            );
        }
    }
    frame.render_widget(
        Paragraph::new(safe(&wizard.error))
            .style(Style::default().fg(color(Color::Yellow)))
            .wrap(Wrap { trim: false }),
        row(area.height.saturating_sub(4), 2),
    );
    frame.render_widget(
        Paragraph::new(if wizard.step == Step::Name {
            t("Enter далее · Esc отменить · Ctrl+C выход")
        } else {
            t("Ctrl+C выход")
        })
        .style(muted),
        row(area.height - 1, 1),
    );
}
async fn prepare_screen(
    root: &Path,
    terminal: &mut ratatui::DefaultTerminal,
    pictures: &mut Pictures,
    ascii: bool,
) -> Result<Option<ProfileKeys>> {
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let root = root.to_owned();
    let mut job = Some(tokio::task::spawn_blocking(move || {
        prepare(&root, |stage| {
            let _ = sender.send(stage);
        })
    }));
    let mut ready = 0;
    let mut prepared = None;
    let mut animation = CreationAnimation::default();
    let mut last_tick = Instant::now();
    // Security uses cells, not inline images. Keep the scene stable while
    // animating so Warp does not clear the screen on every stage or frame.
    pictures.clear_on_change(terminal, (10, 0))?;
    loop {
        while let Ok(stage) = receiver.try_recv() {
            ready = stage;
        }
        if job.as_ref().is_some_and(|job| job.is_finished()) {
            prepared = Some(job.take().unwrap().await??);
            ready = 4;
        }
        let now = Instant::now();
        animation.advance(ready, now.duration_since(last_tick));
        last_tick = now;
        terminal.draw(|frame| draw_animated_preparation(frame, &animation, pictures, ascii))?;
        // Leave the completed key and all four checkmarks visible for 450 ms
        // before revealing Name, as in the Swift registration ceremony.
        if animation.finished() {
            return Ok(prepared);
        }
        if event::poll(Duration::from_millis(33))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && (key.code == KeyCode::Esc || crate::ui::is_quit_key(key))
                {
                    if let Some(job) = job.take() {
                        // Finish temporary-store cleanup before dropping keys.
                        let _ = job.await?;
                    }
                    pictures.clear_graphics(terminal)?;
                    return Ok(None);
                }
            }
        }
        tokio::task::yield_now().await;
    }
}

/// No persistent profile exists until the avatar is confirmed. Cancelling drops the prepared keys.
pub async fn run(
    root: &Path,
    ascii: bool,
    mode: KeyMode,
    settings: Settings,
) -> Result<Option<Created>> {
    run_named(root, ascii, mode, settings, None).await
}

pub async fn run_named(
    root: &Path,
    ascii: bool,
    mode: KeyMode,
    settings: Settings,
    name: Option<&str>,
) -> Result<Option<Created>> {
    settings.validate()?;
    let file_keys = matches!(mode.backend()?, shum_store::vault::KeyBackend::File);
    let mut terminal = ratatui::init();
    let _guard = crate::ui::TerminalGuard;
    let mut pictures = Pictures::new(crate::ui::picture_picker(ascii));
    crossterm::execute!(std::io::stdout(), event::EnableBracketedPaste)?;
    let Some(prepared) = prepare_screen(root, &mut terminal, &mut pictures, ascii).await? else {
        return Ok(None);
    };
    let mut keys = Some(prepared);
    let mut avatar_started = false;
    let mut wizard = Wizard {
        step: Step::Name,
        name: name.unwrap_or_default().into(),
        seed: 0,
        error: String::new(),
        file_keys,
    };
    let mut created = None;
    let mut saving: Option<tokio::task::JoinHandle<Result<Created>>> = None;
    loop {
        if saving.as_ref().is_some_and(|job| job.is_finished()) {
            match saving.take().unwrap().await? {
                Ok(value) => {
                    created = Some(value);
                    wizard.step = Step::Done;
                }
                Err(error) => {
                    wizard.error = error.to_string();
                    let Some(prepared) =
                        prepare_screen(root, &mut terminal, &mut pictures, ascii).await?
                    else {
                        return Ok(None);
                    };
                    wizard.seed = 0;
                    avatar_started = false;
                    keys = Some(prepared);
                    wizard.step = Step::Name;
                }
            }
        }
        pictures.clear_on_change(&mut terminal, (wizard.step as u8, wizard.seed))?;
        terminal.draw(|f| draw(f, &wizard, &mut pictures, ascii))?;
        if !event::poll(Duration::from_millis(50))? {
            tokio::task::yield_now().await;
            continue;
        }
        let ev = event::read()?;
        // Publication is atomic at the store level; let it finish before leaving this screen.
        if wizard.step == Step::Saving {
            continue;
        }
        match ev {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if crate::ui::is_quit_key(key) {
                    pictures.clear_graphics(&mut terminal)?;
                    return Ok(created);
                }
                match (wizard.step, key.code) {
                    (Step::Name, KeyCode::Esc) => {
                        pictures.clear_graphics(&mut terminal)?;
                        return Ok(None);
                    }
                    (Step::Name, KeyCode::Enter) => {
                        let validation = validate_name(&wizard.name);
                        match validation {
                            Ok(()) => {
                                if !avatar_started {
                                    wizard.seed = initial_seed(keys.as_ref().unwrap());
                                    avatar_started = true;
                                }
                                wizard.step = Step::Avatar;
                                wizard.error.clear();
                            }
                            Err(e) => wizard.error = e.to_string(),
                        }
                    }
                    (Step::Name, KeyCode::Backspace) => {
                        wizard.name.pop();
                    }
                    (Step::Name, KeyCode::Char(c))
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            && !c.is_control()
                            && wizard.name.len() + c.len_utf8() <= 64 =>
                    {
                        wizard.name.push(c)
                    }
                    (Step::Avatar, KeyCode::Char('r' | 'к')) => {
                        let mut bytes = [0; 8];
                        getrandom::fill(&mut bytes)?;
                        wizard.seed = u64::from_le_bytes(bytes);
                    }
                    (Step::Avatar, KeyCode::Esc) => wizard.step = Step::Name,
                    (Step::Avatar, KeyCode::Enter) => {
                        wizard.step = Step::Saving;
                        let root = root.to_owned();
                        let name = wizard.name.clone();
                        let settings = settings.clone();
                        let seed = wizard.seed;
                        let keys = keys.take().unwrap();
                        saving = Some(tokio::task::spawn_blocking(move || {
                            create(&root, &name, Some(seed), mode, &settings, keys)
                        }));
                    }
                    (Step::Done, KeyCode::Enter | KeyCode::Esc) => {
                        pictures.clear_graphics(&mut terminal)?;
                        return Ok(created);
                    }
                    _ => {}
                }
            }
            Event::Paste(value) if wizard.step == Step::Name => {
                for c in safe(&value).chars() {
                    if wizard.name.len() + c.len_utf8() <= 64 {
                        wizard.name.push(c);
                    }
                }
            }
            _ => {}
        }
    }
}
pub fn json(created: &Created) -> Result<Value> {
    Ok(
        json!({"profile":created.profile,"card":created.card,"invitation":created.card.invitation()?}),
    )
}
pub fn completion(created: &Created) -> String {
    crate::i18n::format(
        "Профиль «{}» готов.\nНаберите shum, чтобы открыть чаты, или shum --help.",
        &[("0", safe(text(&json!(created.profile.name))).to_string())],
    )
}
