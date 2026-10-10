//! Message selection keeps IDs across live snapshot updates.
use super::{border, centered, reaction_art, safe, stamp, text, Action, View};
use crate::i18n::t;
use anyhow::{bail, Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};
use serde_json::Value;

pub(super) struct Picker {
    contact: String,
    message: String,
    reaction: Option<usize>,
}
impl Picker {
    fn messages<'a>(&self, snapshot: &'a Value) -> Vec<&'a Value> {
        snapshot["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|m| {
                m["contactID"] == self.contact && m["id"].as_str().is_some_and(|id| !id.is_empty())
            })
            .collect()
    }
    fn own_reaction(&self, snapshot: &Value) -> Option<usize> {
        let own = snapshot["profile"]["ownerId"].as_str()?;
        snapshot["reactions"]
            .as_array()?
            .iter()
            .find(|r| r["messageID"] == self.message && r["personID"] == own)
            .and_then(|r| serde_json::from_value(r["mark"]["reaction"].clone()).ok())
            .and_then(|kind| reaction_art::KINDS.iter().position(|k| *k == kind))
    }
}

pub(super) fn open(view: &mut View, snapshot: &Value) -> Result<()> {
    let contact = view.opened.clone().context(t("Сначала откройте чат"))?;
    if !super::contacts(snapshot, 0)
        .iter()
        .any(|c| c["id"] == contact && c["phase"] == "accepted")
    {
        bail!("{}", t("Реакции доступны после принятия приглашения"));
    }
    let mut picker = Picker {
        contact,
        message: String::new(),
        reaction: None,
    };
    picker.message = text(
        &picker
            .messages(snapshot)
            .last()
            .context(t("В этом чате ещё нет сообщений"))?["id"],
    )
    .into();
    view.reactions = Some(picker);
    view.input.clear();
    view.command_mode = false;
    view.status.clear();
    Ok(())
}

pub(super) fn handle(view: &mut View, snapshot: &Value, key: KeyEvent) -> Result<Action> {
    let mut picker = view.reactions.take().expect("active picker");
    let messages = picker.messages(snapshot);
    if messages.is_empty() {
        bail!("{}", t("Сообщения больше недоступны"));
    }
    let position = messages.iter().position(|m| m["id"] == picker.message);
    match (picker.reaction, key.code) {
        (None, KeyCode::Esc) => return Ok(Action::None),
        (Some(_), KeyCode::Esc) => picker.reaction = None,
        (
            None,
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End,
        ) => {
            let i = position.unwrap_or(messages.len() - 1);
            let next = match key.code {
                KeyCode::Up => i.saturating_sub(1),
                KeyCode::PageUp => i.saturating_sub(5),
                KeyCode::Down => (i + 1).min(messages.len() - 1),
                KeyCode::PageDown => (i + 5).min(messages.len() - 1),
                KeyCode::Home => 0,
                _ => messages.len() - 1,
            };
            picker.message = text(&messages[next]["id"]).into();
        }
        (Some(i), KeyCode::Up | KeyCode::Left) => picker.reaction = Some((i + 7) % 8),
        (Some(i), KeyCode::Down | KeyCode::Right) => picker.reaction = Some((i + 1) % 8),
        (Some(_), KeyCode::Home) => picker.reaction = Some(0),
        (Some(_), KeyCode::End) => picker.reaction = Some(7),
        (_, KeyCode::Enter) if position.is_none() => {
            view.reactions = Some(picker);
            bail!(
                "{}",
                t("Сообщение удалено. Выберите другое стрелками или нажмите Esc")
            );
        }
        (None, KeyCode::Enter) => {
            picker.reaction = Some(picker.own_reaction(snapshot).unwrap_or(0))
        }
        (Some(i), KeyCode::Enter) => {
            return Ok(Action::Request(crate::runtime::Request::Reaction {
                message: picker.message,
                reaction: reaction_art::KINDS[i],
            }));
        }
        _ => {}
    }
    view.reactions = Some(picker);
    Ok(Action::None)
}

pub(super) fn popup_area(area: Rect) -> Rect {
    centered(area, 72, 22)
}

pub(super) fn draw(frame: &mut Frame<'_>, snapshot: &Value, view: &View, ascii: bool) {
    let Some(picker) = &view.reactions else {
        return;
    };
    let popup = popup_area(frame.area());
    let title = if picker.reaction.is_some() {
        t("Выберите реакцию")
    } else {
        t("Выберите сообщение")
    };
    frame.render_widget(Clear, popup);
    let block = border(title, ascii);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height < 8 || inner.width < 28 {
        return;
    }
    let accent = if ascii {
        Color::Reset
    } else {
        crate::display::ACCENT
    };
    let selected = Style::default().fg(accent).bg(if ascii {
        Color::Reset
    } else {
        Color::Rgb(20, 48, 29)
    });
    let messages = picker.messages(snapshot);
    let position = messages.iter().position(|m| m["id"] == picker.message);
    let chosen = position.map(|i| messages[i]);
    let preview = chosen
        .map(|m| {
            format!(
                "{} · {}\n{}",
                if m["outgoing"] == true {
                    t("Вы")
                } else {
                    t("Собеседник")
                },
                stamp(&m["timestamp"]),
                safe(text(&m["text"]))
            )
        })
        .unwrap_or_else(|| t("Сообщение удалено. Выберите другое стрелками.").into());
    let footer = if let Some(index) = picker.reaction {
        frame.render_widget(
            Paragraph::new(preview).wrap(Wrap { trim: false }),
            Rect::new(inner.x + 1, inner.y, inner.width - 2, 2),
        );
        let top = inner.y + if inner.height < 12 { 2 } else { 3 };
        let art_height = inner.bottom() - 2 - top;
        let capacity = usize::from(art_height).min(8);
        let start = index.saturating_sub(capacity - 1);
        let label_width = if inner.width >= 44 { 17 } else { 14 };
        for (row, (i, name)) in reaction_art::names()
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .enumerate()
        {
            frame.render_widget(
                Paragraph::new(format!(
                    "{} {}{}",
                    if i == index { ">" } else { " " },
                    name,
                    if picker.own_reaction(snapshot) == Some(i) {
                        " *"
                    } else {
                        ""
                    }
                ))
                .style(if i == index {
                    selected
                } else {
                    Style::default()
                }),
                Rect::new(inner.x + 1, top + row as u16, label_width, 1),
            );
        }
        if !ascii {
            reaction_art::draw(
                frame,
                reaction_art::KINDS[index],
                Rect::new(
                    inner.x + label_width + 2,
                    top,
                    inner.width - label_width - 3,
                    art_height,
                ),
            );
        }
        if picker.own_reaction(snapshot) == Some(index) {
            t("Стрелки: выбор · Enter: снять · Esc: назад")
        } else {
            t("Стрелки: выбор · Enter: поставить · Esc: назад")
        }
    } else {
        let capacity = usize::from(inner.height.saturating_sub(5) / 2).max(1);
        let i = position.unwrap_or(messages.len().saturating_sub(1));
        let start = i.saturating_sub(capacity - 1);
        for (row, (index, message)) in messages
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .enumerate()
        {
            let prefix = if Some(index) == position { ">" } else { " " };
            let name = if message["outgoing"] == true {
                t("Вы")
            } else {
                t("Собеседник")
            };
            let lines = vec![
                Line::from(format!(
                    "{prefix} {name} · {}",
                    stamp(&message["timestamp"])
                )),
                Line::from(Span::raw(format!(
                    "  {}",
                    safe(text(&message["text"])).replace('\n', " ")
                ))),
            ];
            frame.render_widget(
                Paragraph::new(lines).style(if Some(index) == position {
                    selected
                } else {
                    Style::default()
                }),
                Rect::new(inner.x + 1, inner.y + row as u16 * 2, inner.width - 2, 2),
            );
        }
        frame.render_widget(
            Paragraph::new(preview).wrap(Wrap { trim: false }),
            Rect::new(inner.x + 1, inner.bottom() - 4, inner.width - 2, 3),
        );
        t("↑↓: сообщение · Enter: реакции · Esc: отмена")
    };
    let footer = if inner.width < 48 {
        if picker.reaction.is_some() {
            t("↑↓ выбор · Enter · Esc назад")
        } else {
            t("↑↓ выбор · Enter · Esc отмена")
        }
    } else {
        footer
    };
    frame.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}
