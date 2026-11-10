//! QR codes as Slint images: white modules on a transparent background with a
//! 2-module quiet zone, like `generateQRCode` in src/lib/core/utils.ts.

use qrcode::{Color, QrCode};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

const MARGIN: usize = 2;

/// Render `text` at exactly `size` x `size` pixels (show it at that size with
/// `image-rendering: pixelated`). None if the text is too long for a QR code.
pub fn image(text: &str, size: u32) -> Option<Image> {
    let code = QrCode::new(text.as_bytes()).ok()?;
    let width = code.width();
    let colors = code.to_colors();
    let total = width + 2 * MARGIN;
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(size, size);
    let pixels = buffer.make_mut_slice();
    let module = |px: u32| {
        (px as usize * total / size as usize)
            .checked_sub(MARGIN)
            .filter(|m| *m < width)
    };
    for y in 0..size {
        for x in 0..size {
            if let (Some(mx), Some(my)) = (module(x), module(y)) {
                if colors[my * width + mx] == Color::Dark {
                    pixels[(y * size + x) as usize] = Rgba8Pixel {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    };
                }
            }
        }
    }
    Some(Image::from_rgba8(buffer))
}
