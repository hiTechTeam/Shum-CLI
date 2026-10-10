//! Client-side pixel artwork, matching ShumReactionArt.swift (12 × 12).
use crate::i18n::t;
use ratatui::{layout::Rect, style::Color, Frame};
use shum_core::packet::ReactionKind;

pub(super) const KINDS: [ReactionKind; 8] = [
    ReactionKind::Heart,
    ReactionKind::Like,
    ReactionKind::Dislike,
    ReactionKind::Laugh,
    ReactionKind::Fire,
    ReactionKind::Coffin,
    ReactionKind::Hundred,
    ReactionKind::Horror,
];
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

pub(super) fn art(kind: ReactionKind) -> ([&'static str; 12], &'static [(u8, u32)]) {
    match kind {
        ReactionKind::Heart => (
            [
                "............",
                "..RR....RR..",
                ".RPRR..RRRR.",
                "RPRRRRRRRRRR",
                "RRRRRRRRRRRR",
                "RRRRRRRRRRRR",
                ".RRRRRRRRRR.",
                "..RRRRRRRR..",
                "...RRRRRR...",
                "....RRRR....",
                ".....RR.....",
                "............",
            ],
            &[(b'R', 0xFF4245), (b'P', 0xFF9A9C)],
        ),
        ReactionKind::Like => (
            [
                "............",
                "......YY....",
                ".....YYY....",
                ".....YYY....",
                "....YYY.....",
                "GG.YYYYYYYY.",
                "GG.YYYYYYYYY",
                "GG.YYYYYYYO.",
                "GG.YYYYYYYYY",
                "GG.YYYYYYYO.",
                "GG.YYYYYYY..",
                "............",
            ],
            &[(b'G', 0x30D158), (b'Y', 0xFFC93C), (b'O', 0xE59A1C)],
        ),
        ReactionKind::Dislike => (
            [
                "............",
                "GG.YYYYYYY..",
                "GG.YYYYYYYO.",
                "GG.YYYYYYYYY",
                "GG.YYYYYYYO.",
                "GG.YYYYYYYYY",
                "GG.YYYYYYYY.",
                "....YYY.....",
                ".....YYY....",
                ".....YYY....",
                "......YY....",
                "............",
            ],
            &[(b'G', 0x30D158), (b'Y', 0xFFC93C), (b'O', 0xE59A1C)],
        ),
        ReactionKind::Laugh => (
            [
                "....YYYY....",
                "..YYYYYYYY..",
                ".YYYYYYYYYY.",
                ".YDDYYYYDDY.",
                "BYYYYYYYYYYB",
                "BYYYYYYYYYYB",
                "YYDDDDDDDDYY",
                "YYDWWWWWWDYY",
                ".YDMMMMMMDY.",
                ".YYDDDDDDYY.",
                "..YYYYYYYY..",
                "....YYYY....",
            ],
            &[
                (b'Y', 0xFFC93C),
                (b'D', 0x3B2A12),
                (b'W', 0xFFFFFF),
                (b'M', 0x8C2A1E),
                (b'B', 0x5AC8FA),
            ],
        ),
        ReactionKind::Fire => (
            [
                ".....R......",
                ".....RR.....",
                "....RRR.....",
                "...RRRR..R..",
                "..RRROR.RR..",
                "..RROORRRR..",
                ".RROOOORRRR.",
                ".RROOYOOORR.",
                ".RROYYYOORR.",
                ".RROYYYYORR.",
                "..RROYYORR..",
                "...RRRRRR...",
            ],
            &[(b'R', 0xFF4245), (b'O', 0xFF8A1F), (b'Y', 0xFFD23F)],
        ),
        ReactionKind::Coffin => (
            [
                "....DDDD....",
                "...DWWWWD...",
                "..DWWCCWWD..",
                ".DWWWCCWWWD.",
                ".DWCCCCCCWD.",
                ".DWWWCCWWWD.",
                "..DWWCCWWD..",
                "..DWWCCWWD..",
                "..DWWWWWWD..",
                "...DWWWWD...",
                "...DWWWWD...",
                "....DDDD....",
            ],
            &[(b'D', 0x4A2E14), (b'W', 0x8B5A2B), (b'C', 0xE8C9A0)],
        ),
        ReactionKind::Hundred => (
            [
                "............",
                "..R.RRR.RRR.",
                ".RR.R.R.R.R.",
                "..R.R.R.R.R.",
                "..R.R.R.R.R.",
                "..R.R.R.R.R.",
                "..R.RRR.RRR.",
                "............",
                ".RRRRRRRRRR.",
                "............",
                "..RRRRRRRR..",
                "............",
            ],
            &[(b'R', 0xFF4245)],
        ),
        ReactionKind::Horror => (
            [
                "....LLLL....",
                "..LLLLLLLL..",
                ".LLLLLLLLLL.",
                ".YWKYYYYKWY.",
                "YYWWYYYYWWYY",
                "YYYYYYYYYYYY",
                "YYYYYMMYYYYY",
                "YHYYMMMMYYHY",
                "HHYYMMMMYYHH",
                "HH.YYMMYY.HH",
                "....YYYY....",
                "............",
            ],
            &[
                (b'L', 0x7FB2FF),
                (b'Y', 0xFFC93C),
                (b'W', 0xFFFFFF),
                (b'K', 0x2A1A0A),
                (b'M', 0x3B1010),
                (b'H', 0xE59A1C),
            ],
        ),
    }
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
