//! Terminal presentation of Shum's reactions: the sets of stickers that are
//! reactions too, drawn by Shum-Core at 16 or 32 cells and shown here on the
//! terminal's 12 x 12 grid.
use ratatui::{layout::Rect, style::Color, Frame};
use shum_core::packet::ReactionKind;
use std::sync::OnceLock;

/// Every reaction in Shum-Core's order.
pub(super) fn kinds() -> &'static [ReactionKind] {
    static KINDS: OnceLock<Vec<ReactionKind>> = OnceLock::new();
    KINDS.get_or_init(|| shum_core::pool::order().collect())
}

/// Each reaction's emoji and word, in the same order.
pub(super) fn names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        kinds()
            .iter()
            .map(|kind| {
                shum_core::pool::sticker(kind.name())
                    .map(|s| format!("{} {}", s.emoji, s.word))
                    .unwrap_or_else(|| kind.name().into())
            })
            .collect()
    })
}

/// The picture as RGBA bytes and its side.
fn rgba(index: usize) -> (usize, Vec<u8>) {
    let art = shum_core::pool::sticker(kinds()[index].name()).map(|s| &s.art);
    match art {
        Some(art) => (art.size, shum_core::pool::rgba(art)),
        None => (0, Vec::new()),
    }
}

/// The colour at (x, y) of the 12 x 12 grid, taken from the nearest cell.
pub(super) fn pixel(index: usize, x: usize, y: usize) -> Option<Color> {
    let (side, rgba) = rgba(index);
    if side == 0 {
        return None;
    }
    let (sx, sy) = (x * side / 12, y * side / 12);
    let p = &rgba[(sy * side + sx) * 4..][..4];
    (p[3] > 0).then(|| Color::Rgb(p[0], p[1], p[2]))
}

pub(super) fn draw(frame: &mut Frame<'_>, index: usize, area: Rect) {
    let full = area.width >= 24 && area.height >= 12;
    let (width, height) = if full { (24, 12) } else { (12, 6) };
    if area.width < width || area.height < height {
        return;
    }
    let x0 = area.x + (area.width - width) / 2;
    let y0 = area.y + (area.height - height) / 2;
    if full {
        for y in 0..12 {
            for x in 0..12 {
                if let Some(rgb) = pixel(index, x, y) {
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
                    .set_fg(pixel(index, x, y * 2).unwrap_or(background))
                    .set_bg(pixel(index, x, y * 2 + 1).unwrap_or(background));
            }
        }
    }
}

/// Integer enlargement keeps pixel edges sharp in terminals with PNG support.
pub(crate) fn terminal_image(index: usize) -> image::RgbaImage {
    let (side, rgba) = rgba(index);
    let side = side.max(1) as u32;
    let source = image::RgbaImage::from_raw(side, side, rgba)
        .unwrap_or_else(|| image::RgbaImage::new(side, side));
    image::imageops::resize(&source, 192, 192, image::imageops::FilterType::Nearest)
}
