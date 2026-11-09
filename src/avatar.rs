//! Profile pictures: a square PNG of at most `SIZE` px, shared with peers.
//!
//! Local files are cropped to a centred square and scaled down before they
//! ever leave the machine. Pictures received from peers are untrusted:
//! they are size-checked and decoded with tight limits.

use anyhow::{Context, Result, bail};
use image::{ImageFormat, ImageReader, RgbaImage, imageops::FilterType};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Side of the stored and transmitted picture, in pixels.
pub const SIZE: u32 = 160;
/// Largest encoded picture accepted from a peer.
pub const MAX_BYTES: usize = 256 * 1024;
/// Largest source file a user may pick (phone photos are a few MB).
const MAX_SOURCE_BYTES: u64 = 40 * 1024 * 1024;

fn file() -> PathBuf {
    crate::ffmpeg::data_dir().join("avatar.png")
}

/// Turns a picture chosen by the user into the PNG we store and send.
pub fn prepare(path: &Path) -> Result<Vec<u8>> {
    let len = std::fs::metadata(path).context("nie można odczytać pliku")?.len();
    if len > MAX_SOURCE_BYTES {
        bail!("plik jest za duży (maks. 40 MB)");
    }
    let img = ImageReader::open(path)
        .context("nie można otworzyć pliku")?
        .with_guessed_format()?
        .decode()
        .context("to nie jest obsługiwany obraz (JPG, PNG lub WebP)")?;
    let side = img.width().min(img.height());
    let x = (img.width() - side) / 2;
    let y = (img.height() - side) / 2;
    let square = img.crop_imm(x, y, side, side);
    let out = square.resize_exact(SIZE.min(side), SIZE.min(side), FilterType::Lanczos3).to_rgba8();
    encode(&out)
}

fn encode(img: &RgbaImage) -> Result<Vec<u8>> {
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)?;
    if png.len() > MAX_BYTES {
        bail!("zdjęcie po zmniejszeniu jest wciąż za duże");
    }
    Ok(png)
}

/// Decodes a picture from a peer (or from disk). `None` if it is not a
/// small, square-ish PNG.
pub fn decode(png: &[u8]) -> Option<RgbaImage> {
    if png.len() > MAX_BYTES {
        return None;
    }
    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2 * SIZE);
    limits.max_image_height = Some(2 * SIZE);
    limits.max_alloc = Some(4 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode().ok()?.to_rgba8();
    (img.width() > 0 && img.height() > 0).then_some(img)
}

/// The saved picture, if there is a valid one.
pub fn load() -> Option<Vec<u8>> {
    let png = std::fs::read(file()).ok()?;
    decode(&png).map(|_| png)
}

pub fn save(png: Option<&[u8]>) -> Result<()> {
    match png {
        Some(png) => {
            std::fs::create_dir_all(crate::ffmpeg::data_dir())?;
            std::fs::write(file(), png)?;
        }
        None => {
            let _ = std::fs::remove_file(file());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_crops_and_scales() {
        let dir = std::env::temp_dir().join(format!("bibicord-avatar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("wide.jpg");
        image::RgbImage::from_pixel(900, 600, image::Rgb([200, 40, 90])).save(&src).unwrap();
        let png = prepare(&src).unwrap();
        let img = decode(&png).unwrap();
        assert_eq!((img.width(), img.height()), (SIZE, SIZE));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn decode_rejects_junk_and_huge() {
        assert!(decode(b"not a png").is_none());
        let big = RgbaImage::new(2000, 2000);
        let mut png = Vec::new();
        big.write_to(&mut Cursor::new(&mut png), ImageFormat::Png).unwrap();
        assert!(decode(&png).is_none());
    }
}
