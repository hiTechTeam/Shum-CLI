//! Terminal presentation of the v1 pixel avatar drawn by Shum-Core.
pub use shum_core::avatar::{kind, render, render_subject, Avatar, Kind};

/// Supply a sharp PNG to terminals that interpolate small images themselves.
/// Integer nearest-neighbour enlargement preserves every v1 pixel and alpha;
/// it changes client presentation only, not avatar geometry or protocol data.
pub const PNG_SCALE: u32 = 16;
pub fn terminal_image(seed: u64) -> image::RgbaImage {
    let pixels = render_subject(seed).pixels.into_iter().flatten().collect();
    let source = image::RgbaImage::from_raw(36, 36, pixels).expect("36x36 avatar");
    image::imageops::resize(
        &source,
        36 * PNG_SCALE,
        36 * PNG_SCALE,
        image::imageops::FilterType::Nearest,
    )
}
pub const CELL_SIDE: u16 = 12;
pub const CELL_WIDTH: u16 = CELL_SIDE * 2;
/// Font-independent client rendition: two background spaces per square pixel.
/// The seed, palette, features and draw order are shared with the v1 portrait.
pub fn render_cells(seed: u64) -> Vec<[u8; 4]> {
    shum_core::avatar::render_subject_at(seed, usize::from(CELL_SIDE))
}
