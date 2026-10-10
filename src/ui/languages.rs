//! Language selection stays modal without erasing the surrounding conversation.
use super::{border, centered, safe, Action, View, GREEN, MUTED};
use crate::i18n::{self, t};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

pub(super) fn popup_area(area: Rect) -> Rect {
    centered(area, 46, 17)
}
fn code(index: usize) -> &'static str {
    if index == 0 {
        "system"
    } else {
        i18n::LANGUAGES[index - 1].0
    }
}
pub(super) fn open(view: &mut View, from_command: bool) {
    view.languages = Some(
        (0..=i18n::LANGUAGES.len())
            .find(|i| code(*i) == i18n::preference())
            .unwrap_or(0),
    );
    if from_command {
        view.input.clear();
        view.command_mode = false;
    }
    view.status.clear();
}
pub(super) fn handle(view: &mut View, key: KeyEvent) -> Action {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return Action::None;
    }
    let selected = view.languages.as_mut().expect("active language picker");
    let last = i18n::LANGUAGES.len();
    match key.code {
        KeyCode::Up => *selected = selected.saturating_sub(1),
        KeyCode::Down => *selected = (*selected + 1).min(last),
        KeyCode::Home => *selected = 0,
        KeyCode::End => *selected = last,
        KeyCode::PageUp => *selected = selected.saturating_sub(5),
        KeyCode::PageDown => *selected = (*selected + 5).min(last),
        KeyCode::Enter => return Action::Language(code(*selected).into()),
        KeyCode::Esc => view.languages = None,
        _ => {}
    }
    Action::None
}
pub(super) fn draw(frame: &mut Frame<'_>, view: &View, ascii: bool) {
    let Some(selected) = view.languages else {
        return;
    };
    let popup = popup_area(frame.area());
    frame.render_widget(Clear, popup);
    let block = border(t("Язык интерфейса"), ascii).border_style(Style::default().fg(GREEN));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height < 3 {
        return;
    }
    let visible = usize::from(inner.height.saturating_sub(3)).min(i18n::LANGUAGES.len() + 1);
    let start = selected.saturating_sub(visible.saturating_sub(1));
    for (row, index) in (start..=i18n::LANGUAGES.len()).take(visible).enumerate() {
        let name = if index == 0 {
            t("Как в системе")
        } else {
            i18n::LANGUAGES[index - 1].1
        };
        let selected_style = if index == selected {
            Style::default().fg(GREEN).add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let label = format!(
            "{} {:8} {}{}",
            if index == selected { ">" } else { " " },
            code(index),
            name,
            if code(index) == i18n::preference() {
                if ascii {
                    " *"
                } else {
                    " ✓"
                }
            } else {
                ""
            }
        );
        frame.render_widget(
            Paragraph::new(label).style(selected_style),
            Rect::new(inner.x, inner.y + row as u16, inner.width, 1),
        );
    }
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            t("↑↓ выбор · Enter · Esc отмена"),
            Style::default().fg(MUTED),
        ))),
        Rect::new(inner.x, inner.bottom() - 2, inner.width, 1),
    );
    frame.render_widget(
        Paragraph::new(safe(&view.status)),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}
