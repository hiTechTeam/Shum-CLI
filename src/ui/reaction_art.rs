//! Terminal presentation of the reaction artwork drawn by Shum-Core.
use crate::i18n::t;
use ratatui::{layout::Rect, style::Color, Frame};
use shum_core::packet::ReactionKind;
pub(super) use shum_core::reaction_art::{art, KINDS};

pub(super) fn names() -> [&'static str; 8] {
    [
        t("Сердце"),
        t("Нравится"),
        t("Не нравится"),
        t("Смешно"),
        t("Огонь"),
        t("Гроб"),
        "100",
        t("Кошмар"),
    ]
}

pub(super) fn draw(frame: &mut Frame<'_>, kind: ReactionKind, area: Rect) {
    let (rows, colors) = art(kind);
    let full = area.width >= 24 && area.height >= 12;
    let (width, height) = if full { (24, 12) } else { (12, 6) };
    if area.width < width || area.height < height {
        return;
    }
    let x0 = area.x + (area.width - width) / 2;
    let y0 = area.y + (area.height - height) / 2;
    let color = |x: usize, y: usize| {
        colors
            .iter()
            .find(|(symbol, _)| *symbol == rows[y].as_bytes()[x])
            .map(|(_, rgb)| Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, *rgb as u8))
    };
    if full {
        for y in 0..12 {
            for x in 0..12 {
                if let Some(rgb) = color(x, y) {
                    for dx in 0..2 {
                        frame.buffer_mut()[(x0 + x as u16 * 2 + dx, y0 + y as u16)]
                            .set_char(' ')
                            .set_bg(rgb);
                    }
                }
            }
        }
    } else {
        for y in 0..6 {
            for x in 0..12 {
                let cell = &mut frame.buffer_mut()[(x0 + x as u16, y0 + y as u16)];
                let background = if cell.bg == Color::Reset {
                    Color::Rgb(10, 13, 11)
                } else {
                    cell.bg
                };
                cell.set_char('▀')
                    .set_fg(color(x, y * 2).unwrap_or(background))
                    .set_bg(color(x, y * 2 + 1).unwrap_or(background));
            }
        }
    }
}

/// Integer enlargement keeps pixel edges sharp in terminals with PNG support.
pub(crate) fn terminal_image(index: usize) -> image::RgbaImage {
    let (rows, colors) = art(KINDS[index]);
    let source = image::RgbaImage::from_fn(12, 12, |x, y| {
        colors
            .iter()
            .find(|(symbol, _)| *symbol == rows[y as usize].as_bytes()[x as usize])
            .map(|(_, rgb)| image::Rgba([(rgb >> 16) as u8, (rgb >> 8) as u8, *rgb as u8, 255]))
            .unwrap_or(image::Rgba([0, 0, 0, 0]))
    });
    image::imageops::resize(&source, 192, 192, image::imageops::FilterType::Nearest)
}
