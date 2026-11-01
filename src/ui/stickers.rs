//! Choosing a sticker in the open chat, as on the phone: the sets side by
//! side, the chosen set's stickers below with a picture; Enter sends it.
use super::{border, centered, reaction_art, Action, View};
use crate::i18n::t;
use anyhow::{bail, Context, Result};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Clear, Paragraph},
    Frame,
};
use serde_json::Value;
use shum_core::stickers::{Set, SETS};

pub(super) struct Picker {
    contact: String,
    set: usize,
    sticker: usize,
}
impl Picker {
    fn set(&self) -> &'static Set {
        &SETS[self.set]
    }
}

pub(super) fn open(view: &mut View, snapshot: &Value) -> Result<()> {
    let contact = view.opened.clone().context(t("Сначала откройте чат"))?;
    if !super::contacts(snapshot, 0)
        .iter()
        .any(|c| c["id"] == contact && c["phase"] == "accepted")
    {
        bail!("{}", t("Стикеры доступны после принятия приглашения"));
    }
    view.stickers = Some(Picker {
        contact,
        set: 0,
        sticker: 0,
    });
    view.status.clear();
    Ok(())
}

pub(super) fn handle(view: &mut View, key: KeyEvent) -> Result<Action> {
    let mut picker = view.stickers.take().expect("active sticker picker");
    let count = picker.set().stickers.len();
    match key.code {
        KeyCode::Esc => return Ok(Action::None),
        KeyCode::Enter => {
            let sticker = picker.set().stickers[picker.sticker].id.into();
            return Ok(Action::Request(crate::runtime::Request::SendSticker {
                contact: picker.contact,
                sticker,
            }));
        }
        KeyCode::Left | KeyCode::Right => {
            picker.set = if key.code == KeyCode::Left {
                (picker.set + SETS.len() - 1) % SETS.len()
            } else {
                (picker.set + 1) % SETS.len()
            };
            picker.sticker = 0;
        }
        KeyCode::Up => picker.sticker = (picker.sticker + count - 1) % count,
        KeyCode::Down => picker.sticker = (picker.sticker + 1) % count,
        KeyCode::PageUp => picker.sticker = picker.sticker.saturating_sub(8),
        KeyCode::PageDown => picker.sticker = (picker.sticker + 8).min(count - 1),
        KeyCode::Home => picker.sticker = 0,
        KeyCode::End => picker.sticker = count - 1,
        _ => {}
    }
    view.stickers = Some(picker);
    Ok(Action::None)
}

pub(super) fn draw(frame: &mut Frame<'_>, view: &View, ascii: bool) {
    let Some(picker) = &view.stickers else {
        return;
    };
    let popup = centered(frame.area(), 72, 22);
    frame.render_widget(Clear, popup);
    let block = border(t("Стикеры"), ascii);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height < 8 || inner.width < 28 {
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
    let set = picker.set();
    frame.render_widget(
        Paragraph::new(format!(
            "< {} · {}/{} >",
            t(set.title),
            picker.set + 1,
            SETS.len()
        ))
        .style(selected),
        Rect::new(inner.x + 1, inner.y, inner.width - 2, 1),
    );
    let top = inner.y + 2;
    let height = inner.bottom() - 2 - top;
    let capacity = usize::from(height).max(1);
    let start = picker.sticker.saturating_sub(capacity - 1);
    let label_width = if inner.width >= 44 { 22 } else { 16 };
    for (row, (i, sticker)) in set
        .stickers
        .iter()
        .enumerate()
        .skip(start)
        .take(capacity)
        .enumerate()
    {
        frame.render_widget(
            Paragraph::new(format!(
                "{} {} {}",
                if i == picker.sticker { ">" } else { " " },
                sticker.emoji,
                sticker.word
            ))
            .style(if i == picker.sticker {
                selected
            } else {
                Style::default()
            }),
            Rect::new(inner.x + 1, top + row as u16, label_width, 1),
        );
    }
    if !ascii {
        reaction_art::draw_sticker(
            frame,
            set.stickers[picker.sticker].id,
            Rect::new(
                inner.x + label_width + 2,
                top,
                inner.width - label_width - 3,
                height,
            ),
        );
    }
    let footer = if inner.width < 56 {
        t("←→ набор · ↑↓ · Enter · Esc")
    } else {
        t("←→ набор · ↑↓ стикер · Enter: отправить · Esc: назад")
    };
    frame.render_widget(
        Paragraph::new(footer),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn arrows_walk_the_sets_and_enter_sends_the_chosen_sticker() {
        let mut view = View::chat("contact");
        view.stickers = Some(Picker {
            contact: "contact".into(),
            set: 0,
            sticker: 0,
        });
        handle(&mut view, key(KeyCode::Left)).unwrap();
        assert_eq!(view.stickers.as_ref().unwrap().set, SETS.len() - 1);
        handle(&mut view, key(KeyCode::Right)).unwrap();
        handle(&mut view, key(KeyCode::Down)).unwrap();
        let chosen = SETS[0].stickers[1].id;
        match handle(&mut view, key(KeyCode::Enter)).unwrap() {
            Action::Request(crate::runtime::Request::SendSticker { contact, sticker }) => {
                assert_eq!(contact, "contact");
                assert_eq!(sticker, chosen);
            }
            _ => panic!("Enter sends the sticker"),
        }
        assert!(view.stickers.is_none());
    }
}
