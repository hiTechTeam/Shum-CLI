//! Reaction authors are identified by keys, never by display names.
use super::{reaction_art, safe, text, trim_width};
use crate::i18n::t;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

pub(super) const PNG_WIDTH: u16 = 2;
pub(super) const PNG_HEIGHT: u16 = 1;

pub(super) struct Icon {
    pub index: usize,
    pub x: u16,
    pub row: usize,
}

pub(super) fn rows(
    snapshot: &Value,
    message: &Value,
    width: u16,
    ascii: bool,
    png: bool,
) -> (Vec<Line<'static>>, Vec<Icon>) {
    let peer = text(&message["contactID"]);
    let owner = text(&snapshot["profile"]["ownerId"]);
    let contact = snapshot["contacts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"] == peer);
    let participants = [
        (
            peer,
            contact
                .map(|c| text(&c["card"]["name"]))
                .unwrap_or(t("Собеседник")),
        ),
        (owner, snapshot["card"]["name"].as_str().unwrap_or(t("Вы"))),
    ];
    let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
    for (id, name) in participants {
        if id.is_empty() {
            continue;
        }
        let mark = snapshot["reactions"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["messageID"] == message["id"] && r["personID"] == id);
        let Some(kind) =
            mark.and_then(|r| serde_json::from_value(r["mark"]["reaction"].clone()).ok())
        else {
            continue;
        };
        let Some(index) = reaction_art::KINDS.iter().position(|k| *k == kind) else {
            continue;
        };
        let name = safe(name);
        let name = if name.is_empty() { "?".into() } else { name };
        if let Some((_, names)) = groups.iter_mut().find(|(i, _)| *i == index) {
            names.push(name);
        } else {
            groups.push((index, vec![name]));
        }
    }
    let mut lines = Vec::new();
    let mut icons = Vec::new();
    if width == 0 {
        return (lines, icons);
    }
    // PNGs are compact; cell artwork retains the full 12x12 source grid.
    let text_only = ascii || (!png && width < 16);
    let (iw, height) = if text_only {
        (0, 1)
    } else if png {
        (usize::from(PNG_WIDTH), usize::from(PNG_HEIGHT))
    } else {
        (12, 6)
    };
    let style = Style::default().fg(super::MUTED);
    let mut batch: Vec<Line<'static>> = vec![Line::default(); height];
    let mut used = 0;
    for (n, (index, names)) in groups.iter().enumerate() {
        let prefix = if text_only {
            format!("{} ", reaction_art::names()[*index])
        } else {
            String::new()
        };
        let available =
            usize::from(width).saturating_sub(iw + 1 + prefix.width() + if n > 0 { 2 } else { 0 });
        let budget = available.saturating_sub((names.len() - 1) * 3) / names.len();
        let label = format!(
            "{prefix}{}",
            names
                .iter()
                .map(|name| {
                    if name.width() > budget {
                        format!("{}…", trim_width(name, budget.saturating_sub(1)))
                    } else {
                        name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(" / ")
        );
        let label = trim_width(
            &label,
            usize::from(width).saturating_sub(iw + usize::from(iw > 0)),
        );
        let length = iw + usize::from(iw > 0) + label.width();
        let separator = if n > 0 { " / " } else { "" };
        if used > 0 && used + separator.len() + length > usize::from(width) {
            lines.append(&mut batch);
            batch = vec![Line::default(); height];
            used = 0;
        }
        let separator = if used > 0 {
            separator
        } else if n > 0 {
            "/ "
        } else {
            ""
        };
        if png && !text_only {
            icons.push(Icon {
                index: *index,
                x: (used + separator.len()) as u16,
                row: lines.len(),
            });
        }
        for (y, line) in batch.iter_mut().enumerate() {
            line.spans.push(Span::styled(
                if y == height / 2 {
                    separator.to_owned()
                } else {
                    " ".repeat(separator.len())
                },
                style,
            ));
            if !text_only {
                if png {
                    line.spans.push(Span::raw(" ".repeat(iw)));
                } else {
                    line.spans.extend(pixel_row(*index, y));
                }
                line.spans.push(Span::raw(" "));
            }
            line.spans.push(Span::styled(
                if y == height / 2 {
                    label.clone()
                } else {
                    " ".repeat(label.width())
                },
                style,
            ));
        }
        used += separator.len() + length;
    }
    if !groups.is_empty() {
        lines.append(&mut batch);
    }
    if message["outgoing"] == true {
        // Explicit padding keeps PNG placement and text in the same columns.
        for (block, rows) in lines.chunks_mut(height).enumerate() {
            let padding = usize::from(width).saturating_sub(rows[0].width());
            for line in rows {
                line.spans.insert(0, Span::raw(" ".repeat(padding)));
            }
            for icon in icons.iter_mut().filter(|i| i.row == block * height) {
                icon.x += padding as u16;
            }
        }
    }
    (lines, icons)
}

fn pixel_row(index: usize, y: usize) -> Vec<Span<'static>> {
    let (rows, colors) = reaction_art::art(reaction_art::KINDS[index]);
    let color = |x: usize, y: usize| {
        colors
            .iter()
            .find(|(s, _)| *s == rows[y].as_bytes()[x])
            .map(|(_, c)| Color::Rgb((c >> 16) as u8, (c >> 8) as u8, *c as u8))
            .unwrap_or(Color::Rgb(10, 13, 11))
    };
    (0..12)
        .map(|x| {
            let top = color(x, y * 2);
            let bottom = color(x, y * 2 + 1);
            Span::styled(
                if top == bottom { " " } else { "▀" },
                Style::default().fg(top).bg(bottom),
            )
        })
        .collect()
}
