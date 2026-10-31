//! Choosing a photo in the open chat, as reactions are chosen: arrows and
//! Enter save it to Downloads/Shum and open it in the system's viewer.
use super::{border, centered, safe, stamp, text, Action, View};
use crate::i18n::t;
use anyhow::{bail, Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};
use serde_json::Value;
use std::path::PathBuf;

pub(super) struct Picker {
    contact: String,
    message: String,
}

/// The chat's photos that are here in full, oldest first.
fn photos<'a>(snapshot: &'a Value, contact: &str) -> Vec<&'a Value> {
    snapshot["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["contactID"] == contact && m["photo"]["state"] == "ready")
        .collect()
}

pub(super) fn open(view: &mut View, snapshot: &Value) -> Result<()> {
    let contact = view.opened.clone().context(t("Сначала откройте чат"))?;
    let last = photos(snapshot, &contact)
        .last()
        .map(|m| text(&m["id"]).to_owned())
        .context(t("В этом чате нет фото"))?;
    view.photos = Some(Picker {
        contact,
        message: last,
    });
    view.status.clear();
    Ok(())
}

pub(super) fn handle(view: &mut View, snapshot: &Value, key: KeyEvent) -> Result<Action> {
    let mut picker = view.photos.take().expect("active photo picker");
    let list = photos(snapshot, &picker.contact);
    if list.is_empty() {
        bail!("{}", t("В этом чате нет фото"));
    }
    let i = list
        .iter()
        .position(|m| m["id"] == picker.message)
        .unwrap_or(list.len() - 1);
    let next = match key.code {
        KeyCode::Esc => return Ok(Action::None),
        KeyCode::Enter => {
            let message = &list[i];
            return Ok(Action::SavePhoto(
                text(&message["id"]).into(),
                destination(message),
            ));
        }
        KeyCode::Up => i.saturating_sub(1),
        KeyCode::Down => (i + 1).min(list.len() - 1),
        KeyCode::PageUp => i.saturating_sub(5),
        KeyCode::PageDown => (i + 5).min(list.len() - 1),
        KeyCode::Home => 0,
        KeyCode::End => list.len() - 1,
        _ => i,
    };
    picker.message = text(&list[next]["id"]).into();
    view.photos = Some(picker);
    Ok(Action::None)
}

/// Downloads/Shum/Shum <day and time> <short id>.jpg: readable and never
/// the same for two photos.
fn destination(message: &Value) -> PathBuf {
    let folder = directories::UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|d| d.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
        .join("Shum");
    let when = stamp(&message["timestamp"]).replace([':', '/', '.'], "-");
    let short: String = text(&message["id"]).chars().take(8).collect();
    folder.join(format!("Shum {when} {short}.jpg"))
}

/// Shows the saved photo in the system's own viewer.
pub(super) fn show(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let command = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let command = std::process::Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let command = std::process::Command::new("xdg-open").arg(path).spawn();
    let _ = command;
}

pub(super) fn popup_area(area: Rect) -> Rect {
    centered(area, 64, 18)
}

pub(super) fn draw(frame: &mut Frame<'_>, snapshot: &Value, view: &View, ascii: bool) {
    let Some(picker) = &view.photos else {
        return;
    };
    let popup = popup_area(frame.area());
    frame.render_widget(Clear, popup);
    let block = border(t("Фото в чате"), ascii);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height < 4 || inner.width < 20 {
        return;
    }
    let selected = Style::default()
        .fg(if ascii {
            Color::Reset
        } else {
            crate::display::ACCENT
        })
        .bg(if ascii {
            Color::Reset
        } else {
            Color::Rgb(20, 48, 29)
        });
    let list = photos(snapshot, &picker.contact);
    let i = list
        .iter()
        .position(|m| m["id"] == picker.message)
        .unwrap_or(list.len().saturating_sub(1));
    let capacity = usize::from(inner.height.saturating_sub(2)).max(1);
    let start = i.saturating_sub(capacity - 1);
    for (row, (index, m)) in list
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .enumerate()
    {
        let who = if m["outgoing"] == true {
            t("Вы")
        } else {
            t("Собеседник")
        };
        let caption = if m["photo"]["caption"] == true {
            format!(" · {}", safe(text(&m["text"])))
        } else {
            String::new()
        };
        let mark = if index == i { ">" } else { " " };
        frame.render_widget(
            Paragraph::new(Line::raw(format!(
                "{mark} {} · {who}{caption}",
                stamp(&m["timestamp"])
            )))
            .style(if index == i {
                selected
            } else {
                Style::default()
            }),
            Rect::new(inner.x + 1, inner.y + row as u16, inner.width - 2, 1),
        );
    }
    frame.render_widget(
        Paragraph::new(t(
            "Стрелки: выбор · Enter: сохранить и открыть · Esc: назад",
        ))
        .style(Style::default().fg(super::MUTED)),
        Rect::new(inner.x + 1, inner.bottom() - 1, inner.width - 2, 1),
    );
}
