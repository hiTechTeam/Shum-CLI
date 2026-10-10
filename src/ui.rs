mod history_reactions;
mod languages;
pub(crate) mod reaction_art;
mod reactions;
#[cfg(test)]
mod render_tests;

use crate::i18n::t;
use crate::{
    display::{ACCENT as GREEN, LOGO, LOGO_COLOR, MUTED},
    ipc,
    runtime::Request,
    terminal::{safe, text, trim_width},
};
use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{
    picker::Picker,
    protocol::{StatefulProtocol, StatefulProtocolType},
    Resize, StatefulImage,
};
use serde_json::Value;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    path::Path,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct View {
    pub selected: usize,
    pub opened: Option<String>,
    pub input: String,
    pub status: String,
    pub tab: usize,
    pub scroll: u16,
    pub composing: bool,
    pub command_mode: bool,
    pub help: bool,
    info_scroll: u16,
    pub info: Option<(String, String)>,
    pub profile_details: HashMap<String, Value>,
    pub qr: Option<String>,
    pub profiles: Option<Vec<shum_store::profiles::Profile>>,
    pub profile_selected: usize,
    form: Option<Form>,
    reactions: Option<reactions::Picker>,
    languages: Option<usize>,
    chat_rows: Vec<(Rect, String)>,
}
impl View {
    pub fn chat(id: &str) -> Self {
        Self {
            opened: Some(id.into()),
            composing: true,
            ..Self::default()
        }
    }
}
enum Form {
    DeleteProfile { id: String, name: String },
    ClearChat(String),
}

pub struct Pictures {
    picker: Picker,
    pub(crate) colors: crate::display::Colors,
    cache: HashMap<(u64, u16, u16), StatefulProtocol>,
    scene: Option<u64>,
    direct: Option<crate::graphics::DirectImages>,
    popup_covers: Vec<Rect>,
}
impl Pictures {
    pub fn new(picker: Picker) -> Self {
        Self::with_display(picker, crate::display::Display::detect())
    }
    pub fn with_display(picker: Picker, display: crate::display::Display) -> Self {
        let direct = display.direct_images
            && picker.protocol_type() != ratatui_image::picker::ProtocolType::Halfblocks
            && !picker.tmux_detected();
        let mut result = Self::with_colors(picker, display.colors);
        if direct {
            result.direct = Some(crate::graphics::DirectImages::default());
        }
        result
    }
    pub fn with_colors(picker: Picker, colors: crate::display::Colors) -> Self {
        Self {
            picker,
            colors,
            cache: HashMap::new(),
            scene: None,
            direct: None,
            popup_covers: Vec::new(),
        }
    }
    pub(crate) fn cell_avatars(&self) -> bool {
        self.picker.protocol_type() == ratatui_image::picker::ProtocolType::Halfblocks
    }
    pub(crate) fn clear_on_change<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
        scene: impl Hash,
    ) -> std::result::Result<(), B::Error> {
        if self.cell_avatars() {
            return Ok(());
        }
        let mut hash = DefaultHasher::new();
        scene.hash(&mut hash);
        terminal.size()?.hash(&mut hash);
        let next = hash.finish();
        if self.scene != Some(next) {
            self.clear_graphics(terminal)?;
            // Inline graphics live outside Ratatui's cell diff. ECH erases text,
            // but Warp can retain the old image under a transparent replacement.
            // Clear the terminal and its diff buffer together when images move,
            // change, or become covered by a modal. Ordinary typing stays diffed.
            // Terminal::clear queries stdin for the cursor position. We render
            // fullscreen and position the next frame explicitly, so avoid it.
            terminal.backend_mut().clear()?;
            terminal.swap_buffers();
            terminal.swap_buffers();
            self.scene = Some(next);
        }
        Ok(())
    }
    pub(crate) fn clear_graphics<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> std::result::Result<(), B::Error> {
        if let Some(direct) = &mut self.direct {
            direct.clear(terminal)?;
        }
        Ok(())
    }
    pub(crate) fn draw(&mut self, frame: &mut Frame<'_>, seed: u64, area: Rect) {
        self.draw_at(frame, seed, area);
    }
    fn thumbnail(&mut self, frame: &mut Frame<'_>, seed: u64, area: Rect) {
        self.draw_at(frame, seed, area);
    }
    fn draw_at(&mut self, frame: &mut Frame<'_>, seed: u64, area: Rect) {
        let area = area.intersection(frame.area());
        if area.is_empty() {
            return;
        }
        if self.cell_avatars() {
            use crate::avatar::{CELL_SIDE, CELL_WIDTH};
            if area.width < CELL_WIDTH || area.height < CELL_SIDE {
                return;
            }
            let x0 = area.x + (area.width - CELL_WIDTH) / 2;
            let y0 = area.y + (area.height - CELL_SIDE) / 2;
            let pixels = crate::avatar::render_cells(seed);
            for y in 0..CELL_SIDE {
                for x in 0..CELL_SIDE {
                    let pixel = pixels[usize::from(y * CELL_SIDE + x)];
                    if pixel[3] == 0 {
                        continue;
                    }
                    for dx in 0..2 {
                        let cell = &mut frame.buffer_mut()[(x0 + x * 2 + dx, y0 + y)];
                        let background = match cell.bg {
                            Color::Rgb(r, g, b) => [r, g, b],
                            _ => [10, 13, 11],
                        };
                        let rgb = std::array::from_fn::<_, 3, _>(|i| {
                            ((u32::from(pixel[i]) * u32::from(pixel[3])
                                + u32::from(background[i]) * (255 - u32::from(pixel[3]))
                                + 127)
                                / 255) as u8
                        });
                        cell.set_char(' ')
                            .set_bg(Color::Rgb(rgb[0], rgb[1], rgb[2]));
                    }
                }
            }
            return;
        }
        if self.cache.len() > 256 {
            self.cache.clear();
        }
        // Native images are separate terminal objects: a text Clear cannot cover them.
        if covered_by(&self.popup_covers, area) {
            return;
        }
        if let Some(direct) = &mut self.direct {
            direct.draw(frame, seed, area);
            return;
        }
        let state = self
            .cache
            .entry((seed, area.width, area.height))
            .or_insert_with(|| {
                let pixels = crate::avatar::render_subject(seed)
                    .pixels
                    .into_iter()
                    .flatten()
                    .collect();
                let image = image::RgbaImage::from_raw(36, 36, pixels).expect("36x36 avatar");
                self.picker
                    .new_resize_protocol(image::DynamicImage::ImageRgba8(image))
            });
        frame.render_stateful_widget(
            StatefulImage::default()
                .resize(Resize::Scale(Some(image::imageops::FilterType::Nearest))),
            area,
            state,
        );
        if let StatefulProtocolType::ITerm2(image) = state.protocol_type() {
            // ratatui-image encodes pixel bounds using an estimated font size.
            // Cell units keep images inside the layout at any terminal zoom/DPI.
            // https://iterm2.com/documentation-images.html
            let cell = &mut frame.buffer_mut()[(area.x, area.y)];
            if let Some((prefix, dimensions)) = cell.symbol().split_once(";width=") {
                if let Some((_, suffix)) = dimensions.split_once(";doNotMoveCursor=") {
                    let bounded = format!(
                        "{prefix};width={};height={};doNotMoveCursor={suffix}",
                        image.size.width.min(area.width),
                        image.size.height.min(area.height),
                    );
                    cell.set_symbol(&bounded);
                }
            }
        }
    }
}
fn contacts(snapshot: &Value, tab: usize) -> Vec<&Value> {
    snapshot["contacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| match tab {
            1 => c["nearby"] == true,
            2 => crate::invitations::is_invitation(c),
            3 => c["unread"].as_u64().unwrap_or(0) > 0,
            _ => true,
        })
        .collect()
}
fn find_contact<'a>(snapshot: &'a Value, selector: &str) -> Result<&'a Value> {
    crate::terminal::find_contact(snapshot, selector)
}
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}
fn covered_by(covers: &[Rect], area: Rect) -> bool {
    covers
        .iter()
        .any(|popup| !popup.intersection(area).is_empty())
}
fn profile_popup_area(area: Rect, count: usize) -> Rect {
    centered(
        area,
        52,
        count
            .saturating_add(1)
            .saturating_mul(4)
            .saturating_add(5)
            .min(usize::from(area.height)) as u16,
    )
}
fn qr_size(code: &str) -> (u16, u16) {
    (
        code.lines().next().map_or(0, |s| s.chars().count()) as u16,
        code.lines().count() as u16,
    )
}
fn menu_covers(area: Rect, view: &View, ascii: bool, include_profiles: bool) -> Vec<Rect> {
    let mut covers = Vec::new();
    if include_profiles {
        if let Some(profiles) = &view.profiles {
            covers.push(profile_popup_area(area, profiles.len()));
        }
    }
    if view.help || view.info.is_some() {
        covers.push(centered(area, 76, 26));
    }
    if let Some(link) = &view.qr {
        if ascii {
            covers.push(centered(area, 76, 8));
        } else if let Ok(code) = crate::terminal::qr(link, false) {
            let (width, height) = qr_size(&code);
            covers.push(centered(area, width + 4, height + 4));
        }
    }
    if view.reactions.is_some() {
        covers.push(reactions::popup_area(area));
    }
    if view.languages.is_some() {
        covers.push(languages::popup_area(area));
    }
    if view.form.is_some() {
        covers.push(centered(area, 56, 7));
    }
    covers
}
fn border(title: &str, ascii: bool) -> Block<'_> {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(title);
    if ascii {
        block.border_set(ratatui::symbols::border::Set {
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            vertical_left: "|",
            vertical_right: "|",
            horizontal_top: "-",
            horizontal_bottom: "-",
        })
    } else {
        block
    }
}
fn stamp(ms: &Value) -> String {
    chrono::DateTime::from_timestamp_millis(ms.as_i64().unwrap_or(0))
        .map(|d| d.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_default()
}
fn delivery_marker(message: &Value, ascii: bool) -> String {
    if message["outgoing"] != true {
        return String::new();
    }
    let marker = ticks(text(&message["status"]), ascii);
    // Keep the row width fixed from queued through read, including wrap edges.
    if !ascii && ["queued", "forwarding", "delivered", "read"].contains(&text(&message["status"])) {
        format!("{marker:>2}")
    } else {
        marker.into()
    }
}
fn ticks(status: &str, ascii: bool) -> &str {
    match (status, ascii) {
        ("read", false) => "✓✓",
        ("read", true) => "[read]",
        ("delivered", false) => "✓",
        ("delivered", true) => "[delivered]",
        ("forwarding", false) => "↗",
        ("forwarding", true) => "[relay]",
        ("queued", false) => "…",
        ("queued", true) => "[queued]",
        ("expired", _) => t("[истекло]"),
        ("cancelled", _) => t("[отменено]"),
        _ => "",
    }
}

pub fn draw(
    frame: &mut Frame<'_>,
    snapshot: &Value,
    view: &mut View,
    pictures: &mut Pictures,
    ascii: bool,
) {
    pictures.popup_covers = menu_covers(frame.area(), view, ascii, true);
    draw_content(frame, snapshot, view, pictures, ascii);
    pictures.popup_covers.clear();
    pictures.colors.apply(frame.buffer_mut(), ascii);
}
fn draw_content(
    frame: &mut Frame<'_>,
    snapshot: &Value,
    view: &mut View,
    pictures: &mut Pictures,
    ascii: bool,
) {
    let area = frame.area();
    if area.width < 35 || area.height < 12 {
        frame.render_widget(
            Paragraph::new(t("Увеличьте окно терминала (от 35×12). Ctrl+C: выход")),
            area,
        );
        return;
    }
    if !ascii {
        frame.render_widget(
            Paragraph::new("").style(Style::default().bg(Color::Rgb(10, 13, 11))),
            area,
        );
    }
    let accent = if ascii { Color::Reset } else { GREEN };
    let muted = if ascii { Color::Reset } else { MUTED };
    let style = Style::default().fg(accent);
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(if view.command_mode { 5 } else { 2 }),
    ])
    .split(area);
    let relay = snapshot["relays"].as_array().map_or(0, Vec::len);
    let all = contacts(snapshot, 0);
    let nearby = all.iter().filter(|c| c["nearby"] == true).count();
    frame.render_widget(
        Paragraph::new(crate::i18n::format(
            " Shum  {}   релеев {relay}   рядом {nearby}   {}",
            &[
                ("0", safe(text(&snapshot["card"]["name"])).to_string()),
                ("relay", format!("{}", relay)),
                ("nearby", format!("{}", nearby)),
                ("1", format!("{}", chrono::Local::now().format("%H:%M"))),
            ],
        ))
        .style(style),
        vertical[0],
    );
    let counts = [
        all.len(),
        nearby,
        contacts(snapshot, 2).len(),
        contacts(snapshot, 3).len(),
    ];
    let tabs = [t("Все"), t("Рядом"), t("Приглашения"), t("Непрочитанные")]
        .iter()
        .enumerate()
        .map(|(i, label)| {
            Span::styled(
                format!(" {} {} ", label, counts[i]),
                if i == view.tab {
                    if ascii {
                        style
                    } else {
                        style.bg(Color::Rgb(20, 48, 29))
                    }
                } else {
                    Style::default().fg(muted)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), vertical[1]);
    view.chat_rows.clear();
    let listed = contacts(snapshot, view.tab);
    view.selected = view.selected.min(listed.len().saturating_sub(1));
    let full_empty = listed.is_empty() && view.opened.is_none();
    let columns = if pictures.cell_avatars() && !ascii && area.width >= 80 {
        Layout::horizontal([
            Constraint::Length((area.width * 36 / 100).max(40)),
            Constraint::Min(1),
        ])
        .split(vertical[2])
    } else {
        Layout::horizontal([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(vertical[2])
    };
    let chat_area = if full_empty || area.width < 60 {
        vertical[2]
    } else {
        columns[1]
    };
    if !full_empty && (area.width >= 60 || view.opened.is_none()) {
        let list_area = if area.width < 60 {
            vertical[2]
        } else {
            columns[0]
        };
        let block = border(
            if view.tab == 1 {
                t("Рядом")
            } else {
                t("Чаты")
            },
            ascii,
        );
        let inner = block.inner(list_area);
        frame.render_widget(block, list_area);
        let show_avatars = !ascii
            && (!pictures.cell_avatars()
                || (inner.height >= crate::avatar::CELL_SIDE && inner.width >= 36));
        let row_height = if !show_avatars {
            2
        } else if pictures.cell_avatars() {
            crate::avatar::CELL_SIDE
        } else {
            3
        };
        let visible = usize::from(inner.height / row_height).max(1);
        let start = view.selected.saturating_sub(visible - 1);
        for (index, c) in listed.iter().enumerate().skip(start).take(visible) {
            let row = Rect::new(
                inner.x,
                inner.y + (index - start) as u16 * row_height,
                inner.width,
                row_height.min(inner.height),
            );
            let active = index == view.selected;
            let row_style = if active && !ascii {
                Style::default().bg(Color::Rgb(20, 48, 29))
            } else {
                Style::default()
            };
            frame.render_widget(Paragraph::new("").style(row_style), row);
            let avatar_width = (if pictures.cell_avatars() {
                crate::avatar::CELL_WIDTH
            } else {
                6
            })
            .min(row.width);
            let inset = (if !show_avatars { 2 } else { avatar_width + 1 }).min(row.width);
            if show_avatars {
                if let Some(seed) = c["card"]["avatarSeed"].as_u64() {
                    pictures.thumbnail(
                        frame,
                        seed,
                        Rect::new(row.x, row.y, avatar_width, row.height),
                    );
                }
            }
            let name = format!(
                "{}{}{}",
                if active && ascii { "> " } else { "" },
                safe(text(&c["card"]["name"])),
                if c["unread"].as_u64().unwrap_or(0) > 0 {
                    format!(" ({})", c["unread"])
                } else {
                    String::new()
                }
            );
            let last = snapshot["messages"]
                .as_array()
                .into_iter()
                .flatten()
                .rev()
                .find(|m| m["contactID"] == c["id"]);
            let preview = if c["phase"] == "incomingPending" {
                t("Приглашение: Enter").into()
            } else if c["phase"] == "declinedLocally" {
                t("Вы отклонили приглашение.").into()
            } else if c["typing"] == true {
                t("печатает…").into()
            } else if view.tab == 1 {
                nearby_label(c)
            } else {
                last.map(|m| safe(text(&m["text"]))).unwrap_or_default()
            };
            let text_area = Rect::new(
                row.x + inset,
                row.y,
                row.width.saturating_sub(inset),
                row.height,
            );
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(
                        trim_width(&name, text_area.width as usize),
                        if active { style } else { Style::default() },
                    )),
                    Line::from(Span::styled(
                        trim_width(&preview, text_area.width as usize),
                        Style::default().fg(if c["typing"] == true { accent } else { muted }),
                    )),
                ])
                .style(row_style),
                text_area,
            );
            view.chat_rows.push((row, text(&c["id"]).into()));
        }
    }
    if full_empty || (view.opened.is_none() && area.width >= 60) {
        frame.render_widget(border("", ascii), chat_area);
        let compact = ascii || chat_area.height < 19;
        let lines = if compact {
            vec![Line::from("Shum"), Line::from("")]
        } else {
            LOGO.iter()
                .map(|line| {
                    Line::from(Span::styled(
                        line.replace('#', "██").replace('.', "  "),
                        Style::default().fg(LOGO_COLOR),
                    ))
                })
                .collect::<Vec<_>>()
        };
        let mut lines = lines;
        lines.extend([
            Line::from(""),
            Line::from(if full_empty && view.tab == 1 {
                t("Пока никого рядом")
            } else if full_empty {
                t("Пока нет чатов")
            } else {
                t("Выберите чат слева")
            }),
            Line::from(Span::styled(
                if full_empty && view.tab == 1 {
                    t("Откройте Shum на устройстве рядом")
                } else if full_empty {
                    t("Позовите кого-нибудь, и переписка появится здесь")
                } else {
                    t("Enter открыть · ↑↓ выбрать")
                },
                Style::default().fg(muted),
            )),
            Line::from(""),
            Line::from(t("i     показать мой QR-код")),
            Line::from(t("a     добавить по ссылке или QR")),
            Line::from(t("^n    кто рядом по Bluetooth")),
            Line::from(""),
            Line::from(Span::styled(
                t("или в терминале: shum qr · shum add <ссылка>"),
                Style::default().fg(muted),
            )),
        ]);
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Center),
            centered(
                chat_area,
                chat_area.width.saturating_sub(2),
                if compact { 11 } else { 21 },
            ),
        );
    } else if let Some(id) = &view.opened {
        let card = find_contact(snapshot, id).ok();
        let title = card
            .map(|c| safe(text(&c["card"]["name"])))
            .unwrap_or_else(|| t("Чат").into());
        let parts = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(chat_area);
        let block = border("", ascii);
        let inner = block.inner(parts[0]);
        frame.render_widget(block, parts[0]);
        let show_avatar = !ascii
            && (!pictures.cell_avatars()
                || (inner.height >= crate::avatar::CELL_SIDE + 3 && inner.width >= 36));
        let header_height = if !show_avatar {
            2
        } else if pictures.cell_avatars() {
            crate::avatar::CELL_SIDE
        } else {
            4
        }
        .min(inner.height.saturating_sub(1));
        let avatar_width = header_height * 2;
        if let Some(card) = card {
            if show_avatar {
                if let Some(seed) = card["card"]["avatarSeed"].as_u64() {
                    pictures.draw(
                        frame,
                        seed,
                        Rect::new(
                            inner.x,
                            inner.y,
                            avatar_width.min(inner.width),
                            header_height,
                        ),
                    );
                }
            }
            let x = if show_avatar { avatar_width + 1 } else { 0 };
            let phase = if card["typing"] == true {
                t("печатает…")
            } else if card["phase"] != "accepted" {
                crate::invitations::hint(text(&card["phase"]))
            } else if card["online"] == true {
                t("в чате")
            } else {
                t("сквозное шифрование")
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(title.clone(), style)),
                    Line::from(Span::styled(
                        if card["nearby"] == true {
                            format!("{} · {phase}", nearby_label(card))
                        } else {
                            phase.into()
                        },
                        Style::default().fg(muted),
                    )),
                    Line::from(safe(text(&card["card"]["bio"]))),
                ])
                .wrap(Wrap { trim: false }),
                Rect::new(
                    inner.x + x,
                    inner.y,
                    inner.width.saturating_sub(x),
                    header_height,
                ),
            );
        }
        let history = Rect::new(
            inner.x,
            inner.y + header_height,
            inner.width,
            inner.height.saturating_sub(header_height),
        );
        let mut lines = Vec::new();
        let mut reaction_icons = Vec::new();
        let mut visual_rows = 0;
        for m in crate::invitations::timeline(snapshot, id) {
            if m["kind"].is_string() {
                let line = Line::from(Span::styled(
                    format!(
                        "{}  {}",
                        crate::invitations::label(m),
                        stamp(&m["timestamp"])
                    ),
                    Style::default().fg(muted),
                ));
                visual_rows += Paragraph::new(line.clone())
                    .wrap(Wrap { trim: false })
                    .line_count(history.width)
                    + 1;
                lines.push(line);
                lines.push(Line::from(""));
                continue;
            }
            let message_start = lines.len();
            let own = m["outgoing"] == true;
            if let Some(reply) = m["reply"].as_object() {
                lines.push(Line::from(Span::styled(
                    format!("> {}", safe(text(&reply["text"]))),
                    Style::default().fg(muted),
                )));
            }
            let content = safe(text(&m["text"]));
            let meta = format!(" {} {}", stamp(&m["timestamp"]), delivery_marker(m, ascii));
            let line = Line::from(vec![
                Span::raw(content),
                Span::styled(meta, Style::default().fg(if own { accent } else { muted })),
            ]);
            lines.push(if own {
                line.alignment(Alignment::Right)
            } else {
                line
            });
            visual_rows += Paragraph::new(lines[message_start..].to_vec())
                .wrap(Wrap { trim: false })
                .line_count(history.width);
            let (mut reaction_lines, icons) = history_reactions::rows(
                snapshot,
                m,
                history.width,
                ascii,
                pictures.direct.is_some(),
            );
            for mut icon in icons {
                icon.row += visual_rows;
                reaction_icons.push(icon);
            }
            visual_rows += reaction_lines.len() + 1;
            lines.append(&mut reaction_lines);
            lines.push(Line::from(""));
        }
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total = paragraph.line_count(history.width);
        let max_scroll = total.saturating_sub(history.height as usize);
        // Discard invisible overscroll so reversing at the top moves immediately.
        // Recalculate after wrapping/resizing and snapshot updates for mouse and keys alike.
        view.scroll = view.scroll.min(max_scroll.min(u16::MAX as usize) as u16);
        let offset = max_scroll
            .saturating_sub(view.scroll as usize)
            .min(u16::MAX as usize) as u16;
        frame.render_widget(paragraph.scroll((offset, 0)), history);
        if let Some(direct) = &mut pictures.direct {
            for icon in reaction_icons {
                use history_reactions::{PNG_HEIGHT, PNG_WIDTH};
                // Never allow a PNG to cross the history border or input.
                if icon.row >= usize::from(offset)
                    && icon.row + usize::from(PNG_HEIGHT)
                        <= usize::from(offset) + usize::from(history.height)
                {
                    let area = Rect::new(
                        history.x + icon.x,
                        history.y + (icon.row - usize::from(offset)) as u16,
                        PNG_WIDTH,
                        PNG_HEIGHT,
                    );
                    if !covered_by(&pictures.popup_covers, area) {
                        direct.reaction(frame, icon.index, area);
                    }
                }
            }
        }
        let input = Paragraph::new(if view.command_mode {
            ""
        } else {
            view.input.as_str()
        })
        .block(
            border(
                if view.composing {
                    t("Сообщение · Enter отправить · Esc к списку")
                } else {
                    t("Сообщение · Tab ввод")
                },
                ascii,
            )
            .border_style(if view.composing {
                style
            } else {
                Style::default()
            }),
        );
        let width = parts[1].width.saturating_sub(3) as usize;
        let input_width = unicode_width::UnicodeWidthStr::width(view.input.as_str());
        frame.render_widget(
            input.scroll((
                0,
                input_width.saturating_sub(width).min(u16::MAX as usize) as u16,
            )),
            parts[1],
        );
        if view.composing
            && !view.command_mode
            && view.profiles.is_none()
            && view.qr.is_none()
            && view.form.is_none()
            && view.languages.is_none()
            && view.reactions.is_none()
        {
            frame.set_cursor_position((
                parts[1].x + 1 + input_width.min(width) as u16,
                parts[1].y + 1,
            ));
        }
    }
    let footer = vertical[3];
    let footer_parts = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(footer);
    if view.command_mode {
        let width = footer_parts[0].width.saturating_sub(3) as usize;
        let input_width = unicode_width::UnicodeWidthStr::width(view.input.as_str());
        frame.render_widget(
            Paragraph::new(view.input.as_str())
                .scroll((0, input_width.saturating_sub(width) as u16))
                .block(
                    border(t("Команда · /help список · Esc отменить"), ascii).border_style(style),
                ),
            footer_parts[0],
        );
        if view.form.is_none()
            && view.qr.is_none()
            && !view.help
            && view.languages.is_none()
            && view.reactions.is_none()
        {
            frame.set_cursor_position((
                footer_parts[0].x + 1 + input_width.min(width) as u16,
                footer_parts[0].y + 1,
            ));
        }
    }
    let nearby_status = bluetooth_status(snapshot);
    let status = if view.status.is_empty() && view.tab == 1 {
        &nearby_status
    } else if view.status.is_empty() {
        snapshot["error"]
            .as_str()
            .or(snapshot["pushError"].as_str())
            .unwrap_or("")
    } else {
        &view.status
    };
    let hint = if view.composing || view.command_mode {
        t("Enter отправить · ^R реакция · Esc к списку · ^P профили · ^Q выход")
    } else if area.width < 60 {
        t("i QR · a добавить · ^P профили · q выход")
    } else if full_empty {
        t("i мой QR · a добавить · ^N рядом · ^P профили · / команды · q выход")
    } else {
        t("↑↓ чаты · Enter открыть · Tab ввод · / команды · ^P профили · q выход")
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(safe(status)),
            Line::from(Span::styled(hint, Style::default().fg(muted))),
        ]),
        footer_parts[1],
    );
    if view.profiles.is_some() || view.help || view.info.is_some() {
        for cell in &mut frame.buffer_mut().content {
            cell.set_fg(if ascii {
                Color::Reset
            } else {
                Color::Rgb(54, 64, 57)
            });
        }
    }
    if let Some(profiles) = &view.profiles {
        // Profile thumbnails belong to this layer; only higher menus cover them.
        let background_covers = std::mem::replace(
            &mut pictures.popup_covers,
            menu_covers(area, view, ascii, false),
        );
        let row_height = 3;
        let stride = row_height + 1;
        let popup = profile_popup_area(area, profiles.len());
        frame.render_widget(Clear, popup);
        let block = border(t(" Профили "), ascii).border_style(style);
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let visible = usize::from(inner.height.saturating_sub(3) / stride).max(1);
        let start = view.profile_selected.saturating_sub(visible - 1);
        for i in start..=(profiles.len()).min(start + visible - 1) {
            let row = Rect::new(
                inner.x + 1,
                inner.y + 1 + (i - start) as u16 * stride,
                inner.width.saturating_sub(2),
                row_height.min(inner.height.saturating_sub(2)),
            );
            let selected = view.profile_selected == i;
            let rowstyle = if selected && !ascii {
                Style::default().bg(Color::Rgb(20, 48, 29))
            } else {
                Style::default()
            };
            frame.render_widget(Paragraph::new("").style(rowstyle), row);
            let label = if let Some(p) = profiles.get(i) {
                let detail = if p.id == snapshot["profile"]["id"] {
                    Some(snapshot)
                } else {
                    view.profile_details.get(&p.id)
                };
                if !ascii && !pictures.cell_avatars() {
                    if let Some(seed) = detail.and_then(|d| d["card"]["avatarSeed"].as_u64()) {
                        pictures.thumbnail(
                            frame,
                            seed,
                            Rect::new(row.x, row.y, row_height * 2, row.height),
                        );
                    }
                }
                format!(
                    "{}{}{}",
                    if selected && ascii { "> " } else { "" },
                    safe(&p.name),
                    if p.id == snapshot["profile"]["id"] {
                        t("  текущий").into()
                    } else {
                        detail
                            .map(|d| {
                                crate::i18n::format(
                                    "  {} чатов",
                                    &[(
                                        "0",
                                        format!(
                                            "{}",
                                            d["chatCount"]
                                                .as_u64()
                                                .unwrap_or_else(|| contacts(d, 0).len() as u64)
                                        ),
                                    )],
                                )
                            })
                            .unwrap_or_default()
                    }
                )
            } else {
                t("+ Создать новый профиль").into()
            };
            let inset = if ascii || pictures.cell_avatars() {
                0
            } else {
                row_height * 2 + 1
            };
            frame.render_widget(
                Paragraph::new(label).style(rowstyle.fg(if selected {
                    accent
                } else {
                    Color::Reset
                })),
                Rect::new(row.x + inset, row.y + 1, row.width.saturating_sub(inset), 1),
            );
            if let Some(profile) = profiles.get(i) {
                if row.height >= 3 {
                    frame.render_widget(
                        Paragraph::new(format!("ID: {}", profile.id)).style(rowstyle.fg(muted)),
                        Rect::new(row.x + inset, row.y + 2, row.width.saturating_sub(inset), 1),
                    );
                }
            }
        }
        pictures.popup_covers = background_covers;
        frame.render_widget(
            Paragraph::new(t("Enter выбрать · d удалить · Esc закрыть"))
                .style(Style::default().fg(muted)),
            Rect::new(
                inner.x + 1,
                inner.bottom().saturating_sub(1),
                inner.width.saturating_sub(2),
                1,
            ),
        );
    }
    if view.help || view.info.is_some() {
        let help = help_text();
        let (title, body) = view
            .info
            .as_ref()
            .map(|(t, b)| (t.as_str(), b.as_str()))
            .unwrap_or((t("Команды в Shum"), help.as_str()));
        let popup = centered(area, 76, 26);
        frame.render_widget(Clear, popup);
        let paragraph = Paragraph::new(body).wrap(Wrap { trim: false });
        view.info_scroll = view.info_scroll.min(
            paragraph
                .line_count(popup.width.saturating_sub(2))
                .saturating_sub(usize::from(popup.height.saturating_sub(2)))
                .min(u16::MAX as usize) as u16,
        );
        frame.render_widget(
            paragraph
                .scroll((view.info_scroll, 0))
                .block(border(title, ascii).border_style(style)),
            popup,
        );
    }
    if let Some(link) = &view.qr {
        if ascii {
            let popup = centered(area, 76, 8);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(crate::i18n::format(
                    "{link}\n\nQR в режиме ASCII: shum qr --ascii\nEsc закрыть",
                    &[("link", link.to_string())],
                ))
                .wrap(Wrap { trim: false })
                .block(border(t("Моё приглашение"), true)),
                popup,
            );
        } else if let Ok(code) = crate::terminal::qr(link, false) {
            let (width, height) = qr_size(&code);
            let popup = centered(area, width + 4, height + 4);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                border(t("Мой QR · Esc закрыть"), ascii).border_style(style),
                popup,
            );
            if popup.width >= width + 2 && popup.height >= height + 2 {
                frame.render_widget(
                    Paragraph::new(code).style(Style::default().fg(Color::Black).bg(Color::White)),
                    centered(popup, width, height),
                );
            } else {
                frame.render_widget(
                    Paragraph::new(t("Увеличьте окно для QR или выполните shum qr"))
                        .wrap(Wrap { trim: false }),
                    border("", ascii).inner(popup),
                );
            }
        }
    }
    reactions::draw(frame, snapshot, view, ascii);
    languages::draw(frame, view, ascii);
    if let Some(form) = &view.form {
        let title = match form {
            Form::DeleteProfile { .. } => t("Введите имя удаляемого профиля"),
            Form::ClearChat(_) => t("Очистить чат здесь? Введите да"),
        };
        let popup = centered(area, 56, 7);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(crate::i18n::format(
                "{}{}\n{}\nEnter подтвердить · Esc отменить",
                &[
                    (
                        "0",
                        (match form {
                            Form::DeleteProfile { id, name } => format!("{}\n{}\n", safe(name), id),
                            Form::ClearChat(_) => String::new(),
                        })
                        .to_string(),
                    ),
                    ("1", safe(&view.input).to_string()),
                    ("2", safe(&view.status).to_string()),
                ],
            ))
            .block(border(title, ascii).border_style(style)),
            popup,
        );
    }
}

/// Restore the terminal on every return path, including errors in a modal.
pub(crate) struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EndSynchronizedUpdate,
            event::DisableMouseCapture,
            event::DisableBracketedPaste
        );
        ratatui::restore();
    }
}
pub(crate) fn picture_picker(ascii: bool) -> Picker {
    // Querying stdin here consumed early keystrokes in terminals without replies.
    // Use known graphics protocols; other terminals get background-cell portraits.
    let mut picker = Picker::halfblocks();
    if !ascii {
        picker.set_protocol_type(crate::display::Display::detect().images);
    }
    picker
}
pub fn is_quit_key(key: KeyEvent) -> bool {
    key.code == KeyCode::F(10)
        || (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'q' | 'с' | 'й')))
}
fn shortcut(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(c) => KeyCode::Char(match c {
            'й' => 'q',
            'ш' => 'i',
            'ф' => 'a',
            'о' => 'j',
            'л' => 'k',
            'з' => 'p',
            'т' => 'n',
            'в' => 'd',
            'к' => 'r',
            'д' => 'l',
            _ => c,
        }),
        _ => code,
    }
}
pub enum Action {
    None,
    Quit,
    Qr,
    Profiles,
    NewProfile,
    Switch(String),
    Delete(String),
    Request(Request),
    Open(String),
    Tab(usize),
    Help,
    Language(String),
    Languages,
    Info(String, String),
}
fn open_contact(view: &mut View, id: String) {
    view.opened = Some(id);
    view.composing = true;
    view.command_mode = false;
    view.input.clear();
    view.scroll = 0;
}
/// Navigation never writes into the editor. Text shortcuts are active only outside editors.
pub fn handle_key(view: &mut View, snapshot: &Value, key: KeyEvent) -> Result<Action> {
    if key.kind != event::KeyEventKind::Press {
        return Ok(Action::None);
    }
    if is_quit_key(key) {
        return Ok(Action::Quit);
    }
    if view.languages.is_some() {
        return Ok(languages::handle(view, key));
    }
    if view.reactions.is_some() {
        return reactions::handle(view, snapshot, key);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Ok(match shortcut(key.code) {
            KeyCode::Char('r')
                if view.qr.is_none()
                    && !view.help
                    && view.info.is_none()
                    && view.form.is_none()
                    && view.profiles.is_none() =>
            {
                let draft = view.input.clone();
                let command_mode = view.command_mode;
                reactions::open(view, snapshot)?;
                view.input = draft;
                view.command_mode = command_mode;
                Action::None
            }
            KeyCode::Char('l')
                if view.qr.is_none()
                    && !view.help
                    && view.info.is_none()
                    && view.form.is_none()
                    && view.profiles.is_none() =>
            {
                languages::open(view, false);
                Action::None
            }
            KeyCode::Char('p') => Action::Profiles,
            KeyCode::Char('n') => Action::Tab(1),
            _ => Action::None,
        });
    }
    if view.qr.is_some() || view.help || view.info.is_some() {
        if view.help || view.info.is_some() {
            match key.code {
                KeyCode::Down => view.info_scroll = view.info_scroll.saturating_add(1),
                KeyCode::Up => view.info_scroll = view.info_scroll.saturating_sub(1),
                KeyCode::PageDown => view.info_scroll = view.info_scroll.saturating_add(10),
                KeyCode::PageUp => view.info_scroll = view.info_scroll.saturating_sub(10),
                KeyCode::Home => view.info_scroll = 0,
                KeyCode::End => view.info_scroll = u16::MAX,
                _ => {}
            }
        }
        if matches!(
            shortcut(key.code),
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')
        ) {
            view.qr = None;
            view.help = false;
            view.info = None;
            view.info_scroll = 0;
        }
        return Ok(Action::None);
    }
    if let Some(form) = view.form.as_ref() {
        match key.code {
            KeyCode::Esc => {
                view.form = None;
                view.input.clear();
            }
            KeyCode::Enter => match form {
                Form::DeleteProfile { id, name } => {
                    if view.input != *name {
                        bail!(
                            "{}",
                            crate::i18n::format(
                                "Введите точное имя: {}",
                                &[("0", safe(name).to_string())]
                            )
                        );
                    }
                    return Ok(Action::Delete(id.clone()));
                }
                Form::ClearChat(id) => {
                    if view.input != t("да") {
                        bail!("{}", t("Для очистки введите да"));
                    }
                    return Ok(Action::Request(Request::Clear {
                        contact: id.clone(),
                    }));
                }
            },
            KeyCode::Backspace => {
                view.input.pop();
            }
            KeyCode::Char(c) if !c.is_control() && view.input.len() + c.len_utf8() <= 128 => {
                view.input.push(c)
            }
            _ => {}
        }
        return Ok(Action::None);
    }
    if let Some(profiles) = view.profiles.as_ref() {
        match shortcut(key.code) {
            KeyCode::Esc | KeyCode::Char('q') => view.profiles = None,
            KeyCode::Up | KeyCode::Char('k') => {
                view.profile_selected = view.profile_selected.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                view.profile_selected = (view.profile_selected + 1).min(profiles.len())
            }
            KeyCode::Char('n') => return Ok(Action::NewProfile),
            KeyCode::Char('d') => {
                if let Some(p) = profiles.get(view.profile_selected) {
                    view.form = Some(Form::DeleteProfile {
                        id: p.id.clone(),
                        name: p.name.clone(),
                    });
                    view.input.clear();
                }
            }
            KeyCode::Enter => {
                return Ok(profiles
                    .get(view.profile_selected)
                    .map(|p| Action::Switch(p.id.clone()))
                    .unwrap_or(Action::NewProfile))
            }
            _ => {}
        }
        return Ok(Action::None);
    }
    let editing = view.composing || view.command_mode;
    match key.code {
        KeyCode::Esc => {
            if view.command_mode {
                view.command_mode = false;
                view.input.clear();
            } else {
                view.composing = false;
            }
            view.status.clear();
        }
        KeyCode::Tab => {
            if view.command_mode {
                view.command_mode = false;
                view.input.clear();
            } else if view.opened.is_some() {
                view.composing = !view.composing;
            } else {
                view.tab = (view.tab + 1) % 4;
                view.selected = 0;
            }
        }
        KeyCode::PageUp => view.scroll = view.scroll.saturating_add(10),
        KeyCode::PageDown => view.scroll = view.scroll.saturating_sub(10),
        KeyCode::Enter if editing && !view.input.is_empty() => {
            if view.input.trim() == "/react" {
                reactions::open(view, snapshot)?;
                return Ok(Action::None);
            }
            let action = parse_command(&view.input, view.opened.as_deref(), snapshot)?;
            if matches!(action, Action::Languages) {
                languages::open(view, true);
                return Ok(Action::None);
            }
            return Ok(action);
        }
        KeyCode::Enter => {
            if let Some(c) = contacts(snapshot, view.tab).get(view.selected) {
                open_contact(view, text(&c["id"]).into());
            }
        }
        KeyCode::Backspace if editing => {
            view.input.pop();
        }
        KeyCode::Char(c)
            if editing
                && !c.is_control()
                && !key.modifiers.contains(KeyModifiers::ALT)
                && view.input.len() + c.len_utf8() <= 4096 =>
        {
            view.input.push(c)
        }
        _ if !editing => match shortcut(key.code) {
            KeyCode::Char('q') => return Ok(Action::Quit),
            KeyCode::Char('i') => return Ok(Action::Qr),
            KeyCode::Char('a') => {
                view.input = "/add ".into();
                view.command_mode = true;
            }
            KeyCode::Char('/' | '?') => {
                view.input = "/".into();
                view.command_mode = true;
            }
            KeyCode::Char(c @ '1'..='4') => return Ok(Action::Tab((c as u8 - b'1') as usize)),
            KeyCode::Up | KeyCode::Char('k') => view.selected = view.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                view.selected =
                    (view.selected + 1).min(contacts(snapshot, view.tab).len().saturating_sub(1))
            }
            _ => {}
        },
        _ => {}
    }
    Ok(Action::None)
}
fn parse_command(input: &str, current: Option<&str>, snapshot: &Value) -> Result<Action> {
    if !input.starts_with('/') {
        return Ok(Action::Request(Request::Send {
            contact: current.context(t("Сначала выберите чат"))?.into(),
            text: input.into(),
        }));
    }
    let words = shlex::split(input).context(t("Незакрытые кавычки"))?;
    let args = words.iter().map(String::as_str).collect::<Vec<_>>();
    let contact = |arg: Option<&&str>| -> Result<String> {
        arg.copied()
            .or(current)
            .context(t("Укажите ID или откройте чат"))
            .and_then(|id| {
                crate::contact::validate(id)?;
                Ok(id.to_owned())
            })
    };
    let req=match args.as_slice() {
        ["/"|"/help"]=>return Ok(Action::Help),
        ["/q"|"/quit"|"/exit"]=>return Ok(Action::Quit),
        ["/qr"]=>return Ok(Action::Qr),
        ["/invite",..] if args.len()<=2=>Request::Invite{contact:contact(args.get(1))?},
        ["/accept",..] if args.len()<=2=>Request::Accept{contact:contact(args.get(1))?},
        ["/decline",..] if args.len()<=2=>Request::Decline{contact:contact(args.get(1))?},
        ["/read",..] if args.len()<=2=>Request::Read{contact:contact(args.get(1))?},
        ["/clear",..] if args.len()<=2=>Request::Clear{contact:contact(args.get(1))?},
        ["/add",link]=>Request::Add{link:(*link).into()},
        ["/add","--image",path]=>Request::Add{link:crate::terminal::decode_qr(Path::new(path))?},
        ["/open",who]=>return Ok(Action::Open(text(&find_contact(snapshot,who)?["id"]).into())),
        ["/send",who,body @ ..] if !body.is_empty()=>Request::Send{contact:contact(Some(who))?,text:body.join(" ")},
        ["/block",who]=>Request::Block{contact:contact(Some(who))?,blocked:true},
        ["/block",who,"--undo"]=>Request::Block{contact:contact(Some(who))?,blocked:false},
        ["/cancel",message]=>Request::Cancel{message:(*message).into()},
        ["/react",message,reaction]=>Request::Reaction{message:(*message).into(),reaction:serde_json::from_value(Value::String((*reaction).into())).context(t("Реакция: heart like dislike laugh fire coffin hundred horror"))?},
        ["/profile","name",name @ ..] if !name.is_empty()=>Request::Profile{name:Some(name.join(" ")),bio:None,seed:None},
        ["/profile","bio",bio @ ..]=>Request::Profile{name:None,bio:Some(bio.join(" ")),seed:None},
        ["/profile","avatar","--random"]=>{let mut bytes=[0;8];getrandom::fill(&mut bytes)?;Request::Profile{name:None,bio:None,seed:Some(u64::from_le_bytes(bytes))}},
        ["/profile","avatar","--seed",seed]=>Request::Profile{name:None,bio:None,seed:Some(seed.parse().context(t("Семя должно быть целым числом"))?)},
        ["/language",code]=>return Ok(Action::Language((*code).into())),
        ["/language"]=>return Ok(Action::Languages),
        ["/profile"]=>return Ok(Action::Info(t("Профиль").into(),crate::i18n::format("{}\n{}\n\nShum ID: {}\nСетевой ID: {}\n\n/profile name <имя>\n/profile bio <текст>\n/profile avatar --random\nCtrl+P: выбрать или создать профиль", &[("0", safe(text(&snapshot["card"]["name"])).to_string()), ("1", safe(text(&snapshot["card"]["bio"])).to_string()), ("2", text(&snapshot["profile"]["ownerId"]).to_string()), ("3", text(&snapshot["card"]["nostrKey"]).to_string())]))),
        ["/profile","list"]=>return Ok(Action::Profiles),
        ["/chats"|"/contacts"]=>return Ok(Action::Tab(0)),
        ["/chats","--nearby"]|["/nearby"]=>return Ok(Action::Tab(1)),
        ["/chats","--invites"]=>return Ok(Action::Tab(2)),
        ["/chats","--unread"]=>return Ok(Action::Tab(3)),
        ["/status"|"/about"]=>return Ok(Action::Info("Shum".into(),crate::i18n::format("Версия {} · протокол v1\nПрофиль: {}\nРелеев подключено: {}\n{}\nPush API: {}\n{}", &[("0", env!("SHUM_VERSION").to_string()), ("1", safe(text(&snapshot["card"]["name"])).to_string()), ("2", format!("{}", snapshot["relays"].as_array().map_or(0,Vec::len))), ("3", bluetooth_status(snapshot).to_string()), ("4", crate::terminal::push_status(snapshot).to_string()), ("5", safe(text(&snapshot["error"])).to_string())]))),
        ["/keys","verify",who]=>{let c:shum_core::card::Card=serde_json::from_value(find_contact(snapshot,who)?["card"].clone())?;return Ok(Action::Info(t("Сверка ключей").into(),crate::i18n::format("{}\n\nОтпечаток как в iPhone: {}\n\nShum ID: {}", &[("0", safe(&c.name).to_string()), ("1", crate::terminal::fingerprint(&c).to_string()), ("2", c.id().to_string())])));},
        _=>bail!("{}", t("Команда или аргументы не распознаны. /help: список и примеры")),
    };
    Ok(Action::Request(req))
}
fn invitation(snapshot: &Value, size: ratatui::layout::Size) -> Result<String> {
    let card: shum_core::card::Card = serde_json::from_value(snapshot["card"].clone())?;
    let link = card.invitation()?;
    let qr = crate::terminal::qr(&link, false)?;
    if qr.lines().count() + 4 <= size.height as usize
        && qr.lines().next().map_or(0, |s| s.chars().count() + 4) <= size.width as usize
    {
        return Ok(link);
    }
    use base64::Engine;
    Ok(format!(
        "shum://c2/{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hex::decode(&card.nostr_key)?)
    ))
}
struct Pending {
    task: tokio::task::JoinHandle<Result<Option<(String, Value)>>>,
    clear_input: bool,
    input: String,
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct Feed(tokio::task::JoinHandle<()>);
impl Drop for Feed {
    fn drop(&mut self) {
        self.0.abort();
    }
}
#[derive(Clone, PartialEq, Default)]
struct Activity {
    contact: Option<String>,
    typing: bool,
}
fn feed(
    root: &Path,
    profile: &str,
) -> (
    Feed,
    tokio::sync::watch::Receiver<Option<Value>>,
    tokio::sync::watch::Sender<Activity>,
) {
    let root = root.to_owned();
    let profile = profile.to_owned();
    let (tx, rx) = tokio::sync::watch::channel(None);
    let (activity, mut changes) = tokio::sync::watch::channel(Activity::default());
    let job = tokio::spawn(async move {
        let mut previous = Activity::default();
        let mut heartbeat = Instant::now() - Duration::from_secs(30);
        loop {
            let current = changes.borrow_and_update().clone();
            let update = async {
                if current.contact != previous.contact {
                    let _ = ipc::request(
                        &root,
                        &profile,
                        Request::Focus {
                            contact: current.contact.clone(),
                        },
                    )
                    .await;
                }
                if let Some(id) = &current.contact {
                    if heartbeat.elapsed() > Duration::from_secs(4) || current != previous {
                        let _ = ipc::request(
                            &root,
                            &profile,
                            Request::Typing {
                                contact: id.clone(),
                                active: current.typing,
                            },
                        )
                        .await;
                        let _ = ipc::request(
                            &root,
                            &profile,
                            Request::Presence {
                                contact: id.clone(),
                                online: true,
                            },
                        )
                        .await;
                        heartbeat = Instant::now();
                    }
                }
                let value = ipc::request(&root, &profile, Request::Snapshot).await?;
                Ok::<_, anyhow::Error>(value)
            };
            match tokio::time::timeout(Duration::from_secs(2), update).await {
                Ok(Ok(value)) => {
                    previous = current;
                    let _ = tx.send(Some(value));
                }
                _ => {
                    let _ = tx.send(None);
                }
            }
            tokio::select! {_=tokio::time::sleep(Duration::from_millis(250))=>{},result=changes.changed()=>{if result.is_err(){break;}}}
        }
    });
    (Feed(job), rx, activity)
}
pub async fn run(
    root: &Path,
    initial_profile: &str,
    contact: Option<&str>,
    ascii: bool,
) -> Result<()> {
    let mut profile = initial_profile.to_owned();
    let mut snapshot = ipc::request(root, &profile, Request::Snapshot).await?;
    let mut view = View::default();
    if let Some(contact) = contact {
        open_contact(
            &mut view,
            text(&find_contact(&snapshot, contact)?["id"]).into(),
        );
    }
    loop {
        let mut terminal = ratatui::init();
        let guard = TerminalGuard;
        let mut pictures = Pictures::new(picture_picker(ascii));
        crossterm::execute!(
            std::io::stdout(),
            event::EnableMouseCapture,
            event::EnableBracketedPaste
        )?;
        let result = run_loop(
            root,
            &mut profile,
            &mut snapshot,
            &mut view,
            &mut pictures,
            &mut terminal,
            ascii,
        )
        .await;
        let _ = pictures.clear_graphics(&mut terminal);
        drop(guard);
        let _ = tokio::time::timeout(
            Duration::from_millis(300),
            ipc::request(root, &profile, Request::Focus { contact: None }),
        )
        .await;
        if !result? {
            return Ok(());
        }
        let mode = if snapshot["profile"]["keyBackend"] == "file" {
            shum_store::vault::KeyMode::File
        } else {
            shum_store::vault::KeyMode::Auto
        };
        if let Some(created) =
            crate::onboarding::run(root, ascii, mode, crate::onboarding::Settings::default())
                .await?
        {
            profile = created.profile.id;
            ipc::ensure(root, &profile).await?;
            snapshot = ipc::request(root, &profile, Request::Snapshot).await?;
        }
        view = View::default();
    }
}
// Transport receipts and retries change text, not the image layout.
fn history_layout(snapshot: &Value, view: &View, ascii: bool) -> String {
    let messages: Vec<_> = snapshot["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| {
            view.opened
                .as_deref()
                .is_some_and(|id| m["contactID"] == id)
        })
        .collect();
    // With no reaction placements in the history, new text cannot move a PNG.
    // Chat/list avatars occupy fixed rectangles and are tracked separately.
    let has_reactions = snapshot["reactions"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|r| messages.iter().any(|m| m["id"] == r["messageID"]));
    if !has_reactions {
        return String::new();
    }
    let layout: Vec<_> = messages.into_iter().map(|m| serde_json::json!({
        "id": m["id"], "text": m["text"], "timestamp": m["timestamp"],
        "outgoing": m["outgoing"], "replyText": m["reply"]["text"],
        "markerWidth": unicode_width::UnicodeWidthStr::width(delivery_marker(m, ascii).as_str()),
    })).collect();
    serde_json::to_string(&layout).expect("message layout JSON")
}

fn graphics_scene(profile: &str, snapshot: &Value, view: &View, ascii: bool) -> u64 {
    let mut hash = DefaultHasher::new();
    (
        (
            profile,
            view.opened.as_deref(),
            view.tab,
            view.selected,
            view.scroll,
            // Only rendered content/width can move the history images.
            history_layout(snapshot, view, ascii),
            snapshot["reactions"].to_string(),
            snapshot["events"].to_string(),
            snapshot["card"]["name"].to_string(),
        ),
        (
            view.command_mode,
            view.help,
            view.qr.is_some(),
            view.info.is_some(),
            view.profiles.is_some(),
            view.form.is_some(),
            view.reactions.is_some(),
            view.languages.is_some(),
            crate::i18n::current(),
        ),
        contacts(snapshot, view.tab)
            .iter()
            .map(|c| {
                (
                    text(&c["id"]),
                    c["card"]["avatarSeed"].as_u64(),
                    text(&c["card"]["name"]),
                )
            })
            .collect::<Vec<_>>(),
        (
            view.profile_selected,
            view.profiles.as_ref().map(|profiles| {
                profiles
                    .iter()
                    .map(|p| {
                        (
                            p.id.as_str(),
                            view.profile_details
                                .get(&p.id)
                                .and_then(|d| d["card"]["avatarSeed"].as_u64()),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
        ),
    )
        .hash(&mut hash);
    hash.finish()
}

#[allow(clippy::too_many_arguments)]
async fn run_loop(
    root: &Path,
    profile: &mut String,
    snapshot: &mut Value,
    view: &mut View,
    pictures: &mut Pictures,
    terminal: &mut ratatui::DefaultTerminal,
    ascii: bool,
) -> Result<bool> {
    let (mut worker, mut updates, mut activity) = feed(root, profile);
    let mut pending: Option<Pending> = None;
    let mut previews = tokio::task::JoinSet::new();
    loop {
        while let Some(Ok((id, Some(detail)))) = previews.try_join_next() {
            view.profile_details.insert(id, detail);
        }
        if updates.has_changed().unwrap_or(false) {
            if let Some(value) = updates.borrow_and_update().clone() {
                *snapshot = value;
            } else {
                view.status = t("Служба не отвечает. Ctrl+C или F10: выход").into();
            }
        }
        if pending.as_ref().is_some_and(|job| job.task.is_finished()) {
            let mut job = pending.take().unwrap();
            match (&mut job.task).await? {
                Ok(next) => {
                    if let Some((id, next_snapshot)) = next {
                        if id.is_empty() {
                            return Ok(false);
                        }
                        *profile = id;
                        view.opened = None;
                        view.selected = 0;
                        (worker, updates, activity) = feed(root, profile);
                        *snapshot = next_snapshot;
                    }
                    if job.clear_input && view.input == job.input {
                        view.input.clear();
                        view.command_mode = false;
                    }
                    view.form = None;
                    view.profiles = None;
                    view.scroll = 0;
                    view.status = t("Готово").into();
                }
                Err(error) => view.status = error.to_string(),
            }
        }
        use crossterm::SynchronizedUpdate;
        // Keep graphic deletion, screen clearing and the complete next frame in
        // one synchronized update. End is sent even when rendering returns Err.
        std::io::stdout().sync_update(|_| -> std::io::Result<()> {
            pictures.clear_on_change(terminal, graphics_scene(profile, snapshot, view, ascii))?;
            terminal.draw(|f| draw(f, snapshot, view, pictures, ascii))?;
            Ok(())
        })??;
        let action = if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => handle_key(view, snapshot, key),
                Event::Paste(value) => {
                    if view.reactions.is_none()
                        && view.languages.is_none()
                        && (view.composing || view.command_mode || view.form.is_some())
                    {
                        let limit = if view.form.is_some() { 128 } else { 4096 };
                        for c in safe(&value).chars() {
                            if view.input.len() + c.len_utf8() > limit {
                                break;
                            }
                            view.input.push(c);
                        }
                    }
                    Ok(Action::None)
                }
                Event::Mouse(mouse) => {
                    if view.reactions.is_none()
                        && view.languages.is_none()
                        && view.profiles.is_none()
                        && view.form.is_none()
                        && view.qr.is_none()
                        && !view.help
                        && view.info.is_none()
                    {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => view.scroll = view.scroll.saturating_add(3),
                            MouseEventKind::ScrollDown => {
                                view.scroll = view.scroll.saturating_sub(3)
                            }
                            MouseEventKind::Down(event::MouseButton::Left) => {
                                if let Some((_, id)) = view
                                    .chat_rows
                                    .iter()
                                    .find(|(r, _)| r.contains((mouse.column, mouse.row).into()))
                                {
                                    open_contact(view, id.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    Ok(Action::None)
                }
                _ => Ok(Action::None),
            }
        } else {
            Ok(Action::None)
        };
        let action = match action {
            Ok(action) => action,
            Err(error) => {
                view.status = error.to_string();
                Action::None
            }
        };
        match action {
            Action::Quit => return Ok(false),
            Action::NewProfile if pending.is_none() => return Ok(true),
            Action::None | Action::NewProfile => {}
            Action::Qr => match invitation(snapshot, terminal.size()?) {
                Ok(link) => {
                    view.qr = Some(link);
                    if view.command_mode {
                        view.input.clear();
                        view.command_mode = false;
                    }
                }
                Err(error) => view.status = error.to_string(),
            },
            Action::Languages => languages::open(view, true),
            Action::Language(code) => match crate::i18n::save(&code, root) {
                Ok(()) => {
                    view.status = format!("{}: {}", t("Язык интерфейса"), crate::i18n::current());
                    if view.languages.take().is_none() {
                        view.input.clear();
                        view.command_mode = false;
                    }
                }
                Err(error) => view.status = error.to_string(),
            },
            Action::Help => {
                view.help = true;
                view.input.clear();
                view.command_mode = false;
            }
            Action::Info(title, body) => {
                view.info = Some((title, body));
                view.input.clear();
                view.command_mode = false;
            }
            Action::Open(id) => open_contact(view, id),
            Action::Tab(tab) => {
                view.tab = tab;
                view.selected = 0;
                view.composing = false;
                view.command_mode = false;
                view.input.clear();
                if tab == 1 {
                    view.opened = None;
                    view.status.clear();
                }
            }
            Action::Profiles => {
                if view.profiles.is_some() {
                    view.profiles = None;
                } else {
                    let list = shum_store::profiles::Profiles::new(root)?.list()?.1;
                    view.profile_selected = list.iter().position(|p| p.id == *profile).unwrap_or(0);
                    view.profile_details
                        .insert(profile.clone(), snapshot.clone());
                    for p in &list {
                        if p.id != *profile {
                            let root = root.to_owned();
                            let id = p.id.clone();
                            previews.spawn(async move {
                                let detail = profile_preview(&root, &id).await;
                                (id, detail)
                            });
                        }
                    }
                    view.profiles = Some(list);
                }
            }
            Action::Request(Request::Clear { contact })
                if !matches!(view.form, Some(Form::ClearChat(_))) =>
            {
                // Resolve before asking for confirmation, so typos cannot clear a different chat.
                match find_contact(snapshot, &contact) {
                    Ok(c) => {
                        view.form = Some(Form::ClearChat(text(&c["id"]).into()));
                        view.input.clear();
                    }
                    Err(e) => view.status = e.to_string(),
                }
            }
            action => {
                if pending.is_some() {
                    view.status = t("Команда выполняется. Можно выйти: Ctrl+C / F10").into();
                    continue;
                }
                let root = root.to_owned();
                let current = profile.clone();
                let clear_input = !matches!(&action, Action::Request(Request::Reaction { .. }));
                let task = tokio::spawn(async move {
                    match action {
                        Action::Request(request) => {
                            ipc::request(&root, &current, request).await?;
                            Ok(None)
                        }
                        Action::Switch(id) => {
                            ipc::ensure(&root, &id).await?;
                            let _ = tokio::time::timeout(
                                Duration::from_millis(300),
                                ipc::request(&root, &current, Request::Focus { contact: None }),
                            )
                            .await;
                            let snapshot = ipc::request(&root, &id, Request::Snapshot).await?;
                            shum_store::profiles::Profiles::new(&root)?.select(&id)?;
                            Ok(Some((id, snapshot)))
                        }
                        Action::Delete(id) => {
                            ipc::stop(&root, &id).await?;
                            let profiles = shum_store::profiles::Profiles::new(&root)?;
                            profiles.delete(&id)?;
                            if id == current {
                                let next = profiles.list()?.0.unwrap_or_default();
                                let snapshot = if next.is_empty() {
                                    Value::Null
                                } else {
                                    ipc::ensure(&root, &next).await?;
                                    ipc::request(&root, &next, Request::Snapshot).await?
                                };
                                Ok(Some((next, snapshot)))
                            } else {
                                Ok(None)
                            }
                        }
                        _ => Ok(None),
                    }
                });
                pending = Some(Pending {
                    task,
                    clear_input,
                    input: view.input.clone(),
                });
                view.status = t("Выполняется… Ctrl+C: выход").into();
            }
        }
        let next = Activity {
            contact: view.opened.clone(),
            typing: view.composing
                && !view.command_mode
                && !view.input.is_empty()
                && !view.input.starts_with('/'),
        };
        activity.send_if_modified(|old| {
            if *old != next {
                *old = next;
                true
            } else {
                false
            }
        });
        let _ = &worker; // Own the task until this terminal session ends.
        tokio::task::yield_now().await;
    }
}

fn help_text() -> String {
    let rows = [
        ("↑↓ / j k · Enter · Tab", t("Чаты")),
        ("Ctrl+P", t(" Профили ").trim()),
        ("Ctrl+L / /language", t("Язык интерфейса")),
        ("Ctrl+R / /react", t("Выберите сообщение")),
        ("i / /qr", t("Мой QR · Esc закрыть")),
        (
            "a / /add <link> / /add --image <png>",
            t("Добавить контакт по ссылке или QR"),
        ),
        ("Ctrl+N / /chats --nearby", t("Рядом")),
        (
            "/invite [ID]",
            t("Пригласить по Shum ID или полному сетевому ID"),
        ),
        ("/accept [ID]", t("Принять приглашение в чат")),
        ("/decline [ID]", t("Отклонить приглашение")),
        (
            "/open <ID> / /send <ID> <text>",
            t("Отправить сообщение принятому контакту"),
        ),
        ("/read [ID]", t("Отметить чат прочитанным")),
        ("/clear [ID]", t("Очистить чат на этом компьютере")),
        (
            "/react <ID> heart|like|dislike|laugh|fire|coffin|hundred|horror",
            "",
        ),
        ("/cancel <ID>", t("Отменить отправку сообщения")),
        (
            "/block <ID> [--undo]",
            t("Заблокировать контакт; --undo разблокировать"),
        ),
        ("/profile / /profile list", t("Профиль, имя и аватар")),
        ("/profile name <name> / /profile bio <text>", ""),
        ("/profile avatar --random / --seed <number>", ""),
        ("/keys verify <ID>", t("Сверить отпечатки ключей")),
        ("/status", t("Подключения и состояние службы")),
        (
            "1–4 / /chats [--invites|--unread|--nearby]",
            t("Чаты и фильтры"),
        ),
        (
            "/language [system|ru|en|es|zh-Hans|hi|fr|ja|pt-BR|ar|ko]",
            t("Язык интерфейса"),
        ),
        ("/contacts", t("Список контактов и их Shum ID")),
        ("q / /quit / /exit / Ctrl+Q / F10", t("Ctrl+C выход")),
        ("Esc", t("Готово")),
    ];
    rows.into_iter()
        .map(|(keys, label)| {
            if label.is_empty() {
                keys.to_owned()
            } else {
                format!("{keys}  {label}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Read presentation metadata without starting inactive profiles or exposing their messages.
pub async fn profile_preview(root: &Path, id: &str) -> Option<Value> {
    if root.join(id).join("locked").exists() {
        return None;
    }
    if root.join(id).join("daemon.json").exists() {
        return tokio::time::timeout(
            Duration::from_millis(250),
            ipc::request(root, id, Request::Snapshot),
        )
        .await
        .ok()?
        .ok();
    }
    let root = root.to_owned();
    let id = id.to_owned();
    tokio::task::spawn_blocking(move|| {
        let profiles=shum_store::profiles::Profiles::new(root).ok()?;
        let open=profiles.open(Some(&id)).ok()?;
        let state=open.store.state();
        Some(serde_json::json!({"card":state["ownProfileCard"],"chatCount":state["contacts"].as_array().map_or(0,Vec::len)}))
    }).await.ok()?
}

pub fn bluetooth_status(snapshot: &Value) -> String {
    let value = &snapshot["bluetooth"];
    let scan = text(&value["scan"]);
    let advertise = text(&value["advertise"]);
    if value.is_string() {
        return t("Перезапустите службу: shum daemon --stop").into();
    }
    if scan == "other_profile" {
        return t("Рядом показывается другой выбранный профиль").into();
    }
    if scan == "disabled" || value["enabled"] == false {
        return t("Bluetooth отключён · включить: shum --bluetooth status").into();
    }
    if scan == "starting" && advertise == "starting" || scan.is_empty() && advertise.is_empty() {
        return t("Bluetooth: запускается поиск устройств рядом…").into();
    }
    if [scan, advertise]
        .iter()
        .any(|s| s.contains("unauthorized") || s.contains("permission") || s.contains("Permission"))
    {
        return match std::env::consts::OS {
            "macos" => t("Разрешите Shum: Настройки macOS → Конфиденциальность → Bluetooth"),
            "linux" => t("Нет доступа к Bluetooth: проверьте BlueZ и правила D-Bus/Polkit"),
            _ => t("Нет доступа к Bluetooth: проверьте разрешения в настройках системы"),
        }
        .into();
    }
    if scan == "poweredOff" || advertise == "poweredOff" {
        return t("Bluetooth выключен на компьютере").into();
    }
    if scan.contains("adapter not found") || advertise == "unsupported" {
        return t("Bluetooth-адаптер не найден · доступна переписка через релей").into();
    }
    // Windows and Linux find nearby devices but cannot be found yet.
    if advertise.contains("not available on this platform") {
        return if scan == "scanning" {
            t("Bluetooth: поиск устройств рядом включён · этот компьютер другим пока не виден")
        } else {
            t("Bluetooth: запускается поиск устройств рядом…")
        }
        .into();
    }
    if scan == "scanning" && advertise == "advertising" {
        return t("Bluetooth: поиск включён, ваш профиль виден рядом").into();
    }
    crate::i18n::format(
        "Bluetooth · поиск: {} · объявление: {}",
        &[
            (
                "0",
                (if scan.is_empty() {
                    t("запуск")
                } else {
                    scan
                })
                .to_string(),
            ),
            (
                "1",
                (if advertise.is_empty() {
                    t("запуск")
                } else {
                    advertise
                })
                .to_string(),
            ),
        ],
    )
}

pub fn nearby_label(contact: &Value) -> String {
    match contact["distance"].as_u64() {
        Some(meters) => {
            crate::i18n::format("Рядом · ~{meters} м", &[("meters", format!("{}", meters))])
        }
        None => t("Рядом · расстояние неизвестно").into(),
    }
}
