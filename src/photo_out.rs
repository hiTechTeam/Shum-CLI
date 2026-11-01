//! Sending a photo: the system's own file dialog picks it, and it is made
//! small here as the desktop app does (1280 px, JPEG of at most 110 KB)
//! before Shum-Core seals it.
use crate::i18n::t;
use anyhow::{bail, Context, Result};
use base64::Engine as _;
use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Largest JPEG sent, as on the desktop and in the browser.
const MAX_BYTES: usize = 110_000;

/// The photo as Shum-Core takes it.
pub struct Prepared {
    pub base64: String,
    pub width: i64,
    pub height: i64,
}

/// The request that sends `photo` to `contact`, with the text as caption
/// when there is one; otherwise "Фото" stands in for older clients.
pub fn request(contact: String, photo: Prepared, caption: &str) -> crate::runtime::Request {
    let caption = caption.trim();
    crate::runtime::Request::SendPhoto {
        contact,
        photo: photo.base64,
        width: photo.width,
        height: photo.height,
        text: if caption.is_empty() {
            t("Фото").into()
        } else {
            caption.into()
        },
        caption: !caption.is_empty(),
    }
}

/// Opens the system's file dialog for a picture. `None` when it was closed.
pub fn pick() -> Result<Option<PathBuf>> {
    let prompt = t("Фото для Shum");
    #[cfg(target_os = "macos")]
    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "POSIX path of (choose file of type {{\"public.image\"}} with prompt \"{}\")",
            prompt.replace('"', "")
        ))
        .output();
    #[cfg(target_os = "windows")]
    let output = Command::new("powershell")
        .args(["-NoProfile", "-STA", "-Command"])
        .arg(format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $d = New-Object System.Windows.Forms.OpenFileDialog; \
             $d.Title = '{}'; \
             $d.Filter = 'Images|*.jpg;*.jpeg;*.png;*.heic;*.heif'; \
             if ($d.ShowDialog() -eq 'OK') {{ $d.FileName }}",
            prompt.replace('\'', "")
        ))
        .output();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let output = Command::new("zenity")
        .args(["--file-selection", "--title"])
        .arg(prompt)
        .arg("--file-filter=*.jpg *.jpeg *.png *.JPG *.JPEG *.PNG")
        .output()
        .or_else(|_| {
            Command::new("kdialog")
                .args(["--getopenfilename", ".", "image/jpeg image/png", "--title"])
                .arg(prompt)
                .output()
        });
    let output = output.map_err(|_| {
        anyhow::anyhow!(
            "{}",
            t("Окно выбора файла недоступно. Укажите путь: /sendphoto <файл>")
        )
    })?;
    // Closing the dialog is not an error.
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((output.status.success() && !path.is_empty()).then(|| PathBuf::from(path)))
}

/// Reads a JPEG, PNG or (on a Mac) HEIC photo, turns it upright and makes
/// it small enough to send.
pub fn prepare(path: &Path) -> Result<Prepared> {
    let image = read(path)?;
    let (width, height) = (image.width(), image.height());
    let mut side = 1280u32;
    while side >= 320 {
        let scale = (side as f64 / width.max(height) as f64).min(1.0);
        let w = ((width as f64 * scale).round() as u32).max(1);
        let h = ((height as f64 * scale).round() as u32).max(1);
        let small = on_white(&image.resize_exact(w, h, image::imageops::FilterType::Triangle));
        for quality in [80, 70, 60, 50, 42] {
            let mut jpeg = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)
                .encode_image(&small)?;
            if jpeg.len() <= MAX_BYTES {
                return Ok(Prepared {
                    base64: base64::engine::general_purpose::STANDARD.encode(jpeg),
                    width: w.into(),
                    height: h.into(),
                });
            }
        }
        side = side * 4 / 5;
    }
    bail!("{}", t("Фото слишком большое"))
}

fn read(path: &Path) -> Result<DynamicImage> {
    let heic = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("heic") || e.eq_ignore_ascii_case("heif"));
    if heic {
        return read_heic(path);
    }
    let mut decoder = ImageReader::open(path)
        .with_context(|| format!("{} {}", t("Файл не открыть:"), path.display()))?
        .with_guessed_format()?
        .into_decoder()
        .context(t("Это не фото: нужен JPEG или PNG"))?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// iPhone photos are HEIC; the Mac's own sips turns them into JPEG.
#[cfg(target_os = "macos")]
fn read_heic(path: &Path) -> Result<DynamicImage> {
    let out = std::env::temp_dir().join(format!("shum-photo-{}.jpg", std::process::id()));
    let status = Command::new("sips")
        .args(["-s", "format", "jpeg"])
        .arg(path)
        .arg("--out")
        .arg(&out)
        .output()?
        .status;
    if !status.success() {
        bail!("{}", t("Это не фото: нужен JPEG или PNG"));
    }
    let image = read(&out);
    let _ = std::fs::remove_file(&out);
    image
}

#[cfg(not(target_os = "macos"))]
fn read_heic(_path: &Path) -> Result<DynamicImage> {
    bail!("{}", t("HEIC здесь не открыть: сохраните фото как JPEG"))
}

/// Transparent parts become white, as on the desktop.
fn on_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let blend = |c: u8| ((c as u16 * a as u16 + 255 * (255 - a as u16)) / 255) as u8;
        image::Rgb([blend(r), blend(g), blend(b)])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_large_photo_is_made_small_and_keeps_its_proportions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.png");
        // Noise compresses badly, so the size limit is really tested.
        let mut seed = 7u32;
        let image = image::RgbaImage::from_fn(3000, 2000, |_, _| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let [a, b, c, _] = seed.to_le_bytes();
            image::Rgba([a, b, c, 255])
        });
        image.save(&path).unwrap();
        let photo = prepare(&path).unwrap();
        assert!(photo.width <= 1280 && photo.height <= 1280);
        assert_eq!((photo.width * 2) / photo.height, 3);
        let jpeg = base64::engine::general_purpose::STANDARD
            .decode(&photo.base64)
            .unwrap();
        assert!(jpeg.len() <= MAX_BYTES);
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn a_small_photo_keeps_its_size_and_transparency_turns_white() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.png");
        image::RgbaImage::from_pixel(40, 20, image::Rgba([0, 0, 0, 0]))
            .save(&path)
            .unwrap();
        let photo = prepare(&path).unwrap();
        assert_eq!((photo.width, photo.height), (40, 20));
        let jpeg = base64::engine::general_purpose::STANDARD
            .decode(&photo.base64)
            .unwrap();
        let back = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert!(back.get_pixel(20, 10).0.iter().all(|&c| c > 240));
    }

    #[test]
    fn a_file_that_is_not_a_photo_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.txt");
        std::fs::write(&path, "not a photo").unwrap();
        assert!(prepare(&path).is_err());
    }

    #[test]
    fn the_caption_is_the_text_or_photo_stands_in() {
        let photo = || Prepared {
            base64: String::new(),
            width: 1,
            height: 1,
        };
        let with = request("c".into(), photo(), "  на море ");
        let without = request("c".into(), photo(), "  ");
        let json = |r| serde_json::to_value(r).unwrap();
        assert_eq!(json(with)["caption"], true);
        assert_eq!(
            json(request("c".into(), photo(), "на море"))["text"],
            "на море"
        );
        assert_eq!(json(without)["caption"], false);
    }
}
