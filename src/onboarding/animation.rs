//! Client-only presentation of the Swift registration ceremony.
use crate::display::{ACCENT, MUTED};
use ratatui::{style::Color, Frame};
use std::time::Duration;

const STAGE_SECONDS: f64 = 1.5;
const FINISH_SECONDS: f64 = 6.45;

/// A visual timeline gated by completed security operations. A slow or failed
/// operation cannot gain a checkmark merely because its animation has elapsed.
#[derive(Default, Clone, Copy)]
pub struct CreationAnimation {
    elapsed: f64,
}
impl CreationAnimation {
    pub fn advance(&mut self, ready: usize, delta: Duration) {
        let limit = if ready >= 4 {
            FINISH_SECONDS
        } else {
            (ready + 1) as f64 * STAGE_SECONDS - 0.001
        };
        self.elapsed = (self.elapsed + delta.as_secs_f64()).min(limit);
    }
    pub fn completed(&self) -> usize {
        (self.elapsed / STAGE_SECONDS).floor().min(4.0) as usize
    }
    pub fn percent(&self) -> u16 {
        (self.elapsed / (4.0 * STAGE_SECONDS) * 100.0).min(100.0) as u16
    }
    pub fn finished(&self) -> bool {
        self.elapsed >= FINISH_SECONDS
    }
    pub(super) fn settled(completed: usize) -> Self {
        Self {
            elapsed: if completed >= 4 {
                FINISH_SECONDS
            } else {
                completed as f64 * STAGE_SECONDS
            },
        }
    }
    pub(super) fn spinner(&self) -> char {
        ['|', '/', '-', '\\'][(self.elapsed * 8.0) as usize % 4]
    }
}

#[derive(Clone, Copy)]
struct Pixel(i16, i16);
fn horizontal(y: i16, from: i16, through: i16) -> Vec<Pixel> {
    (from..=through).map(|x| Pixel(x, y)).collect()
}
fn vertical(x: i16, from: i16, through: i16) -> Vec<Pixel> {
    (from..=through).map(|y| Pixel(x, y)).collect()
}
fn outline(x: i16, y: i16, width: i16, height: i16) -> Vec<Pixel> {
    let mut pixels = horizontal(y, x + 1, x + width - 2);
    pixels.extend(vertical(x, y + 1, y + height - 2));
    pixels.extend(vertical(x + width - 1, y + 1, y + height - 2));
    pixels.extend(horizontal(y + height - 1, x + 1, x + width - 2));
    pixels
}
fn diagonal(from: Pixel, to: Pixel) -> Vec<Pixel> {
    let steps = (to.0 - from.0).abs().max((to.1 - from.1).abs());
    (0..=steps)
        .map(|step| {
            Pixel(
                from.0 + (to.0 - from.0) * step / steps,
                from.1 + (to.1 - from.1) * step / steps,
            )
        })
        .collect()
}
fn progress(elapsed: f64, start: f64, duration: f64) -> f64 {
    let value = ((elapsed - start) / duration).clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}
fn blend(from: Color, to: Color, amount: f64) -> Color {
    let (Color::Rgb(r, g, b), Color::Rgb(a, c, d)) = (from, to) else {
        return to;
    };
    let channel = |start: u8, end: u8| {
        (f64::from(start) + (f64::from(end) - f64::from(start)) * amount) as u8
    };
    Color::Rgb(channel(r, a), channel(g, c), channel(b, d))
}

// Same pixel geometry, smoothstep easing, reveal order and scatter offsets as
// RegistrationKeyCreationIllustration in RegistrationSecurityViews.swift.
// Whole background cells avoid the block-glyph seams seen in Apple Terminal.
pub(super) fn draw_key(
    frame: &mut Frame<'_>,
    x: u16,
    y: u16,
    height: u16,
    animation: &CreationAnimation,
) {
    let elapsed = animation.elapsed;
    let completion = progress(elapsed, 5.65, 0.65);
    let background = Color::Rgb(10, 13, 11);
    let primary = blend(Color::Rgb(230, 234, 231), ACCENT, completion);
    let mut key = outline(12, 5, 9, 9);
    key.extend(vertical(16, 14, 24));
    key.extend(horizontal(20, 16, 20));
    key.extend(horizontal(23, 16, 19));
    let cloud = [
        Pixel(14, 10),
        Pixel(15, 9),
        Pixel(16, 9),
        Pixel(17, 10),
        Pixel(18, 10),
        Pixel(14, 11),
        Pixel(15, 11),
        Pixel(16, 11),
        Pixel(17, 11),
        Pixel(18, 11),
    ];
    let mut shield = horizontal(2, 8, 24);
    shield.extend([Pixel(7, 3), Pixel(25, 3)]);
    shield.extend(vertical(6, 4, 16));
    shield.extend(vertical(26, 4, 16));
    for row in 17..=25 {
        shield.extend([Pixel(row - 10, row), Pixel(42 - row, row)]);
    }
    shield.push(Pixel(16, 26));
    let mut storage = outline(3, 0, 27, 29);
    for column in (7..=25).step_by(4) {
        storage.extend([
            Pixel(column, 0),
            Pixel(column, -1),
            Pixel(column, 28),
            Pixel(column, 29),
        ]);
    }
    let mut check = diagonal(Pixel(11, 15), Pixel(15, 19));
    check.extend(diagonal(Pixel(15, 19), Pixel(22, 12)));

    let mut paint = |pixel: Pixel, color: Color| {
        // Fit Swift's 32×31 pixel canvas to 32 columns and 13 rows at 80×32.
        if !(0..32).contains(&pixel.0) || !(-1..30).contains(&pixel.1) {
            return;
        }
        let column = x + pixel.0 as u16;
        let row = y + ((pixel.1 + 1) as u16 * height / 31);
        let cell = &mut frame.buffer_mut()[(column, row)];
        cell.set_symbol(" ").set_bg(color);
    };
    let mut layer = |pixels: &[Pixel], reveal: f64, color: Color, scatter: bool| {
        let count = reveal * (pixels.len() + 6) as f64;
        for (index, &pixel) in pixels.iter().enumerate() {
            let local = (count - index as f64).clamp(0.0, 1.0);
            if local == 0.0 {
                continue;
            }
            let offset = |factor: usize, modulo: usize, origin: f64| {
                (((index * factor % modulo) as f64 - origin) * (1.0 - local)).round() as i16
            };
            let position = if scatter {
                Pixel(pixel.0 + offset(17, 13, 6.0), pixel.1 + offset(11, 15, 7.0))
            } else {
                pixel
            };
            paint(position, blend(background, color, local));
        }
    };
    if completion < 1.0 {
        layer(
            &storage,
            progress(elapsed, 2.75, 1.55),
            blend(MUTED, background, completion),
            false,
        );
    }
    layer(&shield, progress(elapsed, 1.25, 1.8), primary, false);
    layer(&key, progress(elapsed, 0.05, 1.35), primary, true);
    layer(&cloud, progress(elapsed, 0.05, 1.35), ACCENT, true);
    let verification = progress(elapsed, 4.35, 1.35);
    if verification > 0.0 && completion < 1.0 {
        let scan_y = (4.0 + 20.0 * verification) as i16;
        layer(&horizontal(scan_y, 6, 26), 1.0, ACCENT, false);
    }
    layer(&check, verification, ACCENT, false);
}
