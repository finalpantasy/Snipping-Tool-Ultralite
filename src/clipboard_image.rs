//! Image clipboard payloads: a fast PNG plus a bottom-up CF_DIBV5 bitmap.
//!
//! Both payloads are built before the clipboard is opened, so other apps never
//! find it emptied and locked while a large capture is being compressed.

/// Byte length of `BITMAPV5HEADER`.
pub const DIBV5_HEADER_LEN: usize = 124;

const BI_BITFIELDS: u32 = 3;
const LCS_SRGB: u32 = 0x7352_4742;
const LCS_GM_IMAGES: u32 = 4;

/// Encode straight RGBA8 pixels as PNG with the fast deflate preset History uses.
pub fn encode_png(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
    .map_err(|e| format!("Failed to encode clipboard PNG: {e}"))?;
    Ok(png)
}

/// Total CF_DIBV5 size (header plus 32-bit pixels), or `None` if Windows cannot describe it.
pub fn dibv5_len(width: u32, height: u32) -> Option<usize> {
    i32::try_from(width).ok()?;
    i32::try_from(height).ok()?;
    let image = u32::try_from(u64::from(width) * u64::from(height) * 4).ok()?;
    (image as usize).checked_add(DIBV5_HEADER_LEN)
}

/// Write a CF_DIBV5 bitmap in the same layout arboard used: positive height
/// (bottom-up rows, which Word and WordPad require), BGRA bit fields and
/// straight alpha. `dst` must be exactly `dibv5_len(width, height)` bytes.
pub fn write_dibv5(dst: &mut [u8], pixels: &[u8], width: u32, height: u32) {
    let row = width as usize * 4;
    let image = row * height as usize;
    assert_eq!(pixels.len(), image, "RGBA buffer does not match dimensions");
    assert_eq!(dst.len(), DIBV5_HEADER_LEN + image, "DIBV5 buffer size");
    let (header, body) = dst.split_at_mut(DIBV5_HEADER_LEN);
    header.fill(0);
    let mut put = |offset: usize, value: u32| {
        header[offset..offset + 4].copy_from_slice(&value.to_le_bytes())
    };
    put(0, DIBV5_HEADER_LEN as u32);
    put(4, width);
    put(8, height);
    put(12, 1 | (32 << 16)); // planes, bits per pixel
    put(16, BI_BITFIELDS);
    put(20, image as u32);
    put(40, 0x00ff_0000);
    put(44, 0x0000_ff00);
    put(48, 0x0000_00ff);
    put(52, 0xff00_0000);
    put(56, LCS_SRGB);
    put(108, LCS_GM_IMAGES);
    if row == 0 {
        return;
    }
    for (out, src) in body
        .chunks_exact_mut(row)
        .zip(pixels.chunks_exact(row).rev())
    {
        for (o, p) in out.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            o.copy_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
}

#[cfg(windows)]
pub use win::write;

#[cfg(windows)]
mod win {
    use windows::core::w;
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_DIBV5: u32 = 17;
    // Matches arboard: one try plus five retries 5 ms apart, as Chromium does.
    const OPEN_RETRIES: usize = 5;

    /// Moveable global memory; freed on drop unless the clipboard took ownership.
    pub(super) struct GlobalBuffer {
        handle: HGLOBAL,
        len: usize,
    }

    impl GlobalBuffer {
        pub(super) fn filled(len: usize, fill: impl FnOnce(&mut [u8])) -> Result<Self, String> {
            let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, len.max(1)) }
                .map_err(|e| format!("Failed to allocate clipboard memory: {e}"))?;
            let buffer = Self { handle, len };
            buffer.with_bytes(fill)?;
            Ok(buffer)
        }

        pub(super) fn with_bytes<T>(
            &self,
            use_bytes: impl FnOnce(&mut [u8]) -> T,
        ) -> Result<T, String> {
            let ptr = unsafe { GlobalLock(self.handle) }.cast::<u8>();
            if ptr.is_null() {
                return Err("Failed to lock clipboard memory".into());
            }
            // SAFETY: the block was allocated with at least `len` bytes and stays
            // locked, so it cannot move, until the unlock below.
            let result = use_bytes(unsafe { std::slice::from_raw_parts_mut(ptr, self.len) });
            // Unlocking to a zero count reports "failure" with no error code.
            let _ = unsafe { GlobalUnlock(self.handle) };
            Ok(result)
        }

        fn publish(self, format: u32) -> Result<(), String> {
            unsafe { SetClipboardData(format, HANDLE(self.handle.0)) }
                .map_err(|e| format!("Failed to copy image to clipboard: {e}"))?;
            // The system owns the memory once SetClipboardData succeeds.
            std::mem::forget(self);
            Ok(())
        }
    }

    impl Drop for GlobalBuffer {
        fn drop(&mut self) {
            let _ = unsafe { GlobalFree(self.handle) };
        }
    }

    pub(super) struct Payload {
        pub(super) png: GlobalBuffer,
        pub(super) dib: GlobalBuffer,
    }

    /// Build both clipboard payloads without touching the clipboard.
    pub(super) fn prepare(pixels: &[u8], width: u32, height: u32) -> Result<Payload, String> {
        let dib_len = super::dibv5_len(width, height)
            .ok_or("Image is too large for the Windows clipboard")?;
        let encoded = super::encode_png(pixels, width, height)?;
        let png = GlobalBuffer::filled(encoded.len(), |dst| dst.copy_from_slice(&encoded))?;
        drop(encoded);
        let dib = GlobalBuffer::filled(dib_len, |dst| {
            super::write_dibv5(dst, pixels, width, height)
        })?;
        Ok(Payload { png, dib })
    }

    struct OpenedClipboard;

    impl OpenedClipboard {
        fn open() -> Result<Self, String> {
            let mut retries = OPEN_RETRIES;
            loop {
                match unsafe { OpenClipboard(HWND::default()) } {
                    Ok(()) => return Ok(Self),
                    Err(e) if retries == 0 => {
                        return Err(format!("Failed to open clipboard: {e}"));
                    }
                    Err(_) => retries -= 1,
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }

    impl Drop for OpenedClipboard {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    /// Publish PNG first (some apps take the first format they recognise),
    /// then CF_DIBV5 for apps that only read bitmaps.
    pub fn write(pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        let payload = prepare(pixels, width, height)?;
        let png_format = unsafe { RegisterClipboardFormatW(w!("PNG")) };
        if png_format == 0 {
            return Err("Cannot register PNG clipboard format".into());
        }
        let _clipboard = OpenedClipboard::open()?;
        unsafe { EmptyClipboard() }.map_err(|e| format!("Failed to empty the clipboard: {e}"))?;
        payload.png.publish(png_format)?;
        payload.dib.publish(CF_DIBV5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageDecoder;

    /// Distinct rows and columns plus varied alpha, so a flipped, channel
    /// swapped or premultiplied result cannot match by accident.
    fn sample(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[
                    (x * 37 + y) as u8,
                    (y * 53) as u8,
                    (x * 11 + y * 7) as u8,
                    [255, 200, 128, 1, 0][((x + y) % 5) as usize],
                ]);
            }
        }
        pixels
    }

    fn decode_png(png: &[u8]) -> (u32, u32, Vec<u8>) {
        let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
            .unwrap()
            .into_rgba8();
        (image.width(), image.height(), image.into_raw())
    }

    /// Decode as a .bmp file. `new_without_file_header` would skip 12 bytes of
    /// trailing masks that a V5 header (here and in arboard) does not carry,
    /// so give the decoder the true pixel offset with a BITMAPFILEHEADER.
    fn decode_dibv5(dib: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut file = b"BM".to_vec();
        file.extend_from_slice(&(14 + dib.len() as u32).to_le_bytes());
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&(14 + DIBV5_HEADER_LEN as u32).to_le_bytes());
        file.extend_from_slice(dib);
        let decoder = image::codecs::bmp::BmpDecoder::new(std::io::Cursor::new(file)).unwrap();
        assert_eq!(decoder.color_type(), image::ColorType::Rgba8);
        let (width, height) = decoder.dimensions();
        let mut pixels = vec![0; decoder.total_bytes() as usize];
        decoder.read_image(&mut pixels).unwrap();
        (width, height, pixels)
    }

    #[test]
    fn png_and_dibv5_round_trip_through_image_decoders() {
        let (width, height) = (7, 5);
        let pixels = sample(width, height);

        assert_eq!(
            decode_png(&encode_png(&pixels, width, height).unwrap()),
            (width, height, pixels.clone())
        );

        let mut dib = vec![0xAA; dibv5_len(width, height).unwrap()];
        write_dibv5(&mut dib, &pixels, width, height);
        assert_eq!(decode_dibv5(&dib), (width, height, pixels.clone()));

        // Positive height means bottom-up storage: the first stored row is the
        // image's last row, in BGRA order with straight alpha.
        assert_eq!(i32::from_le_bytes(dib[8..12].try_into().unwrap()), 5);
        let last_row = &pixels[(width * (height - 1) * 4) as usize..][..4];
        assert_eq!(
            &dib[DIBV5_HEADER_LEN..DIBV5_HEADER_LEN + 4],
            &[last_row[2], last_row[1], last_row[0], last_row[3]]
        );
    }

    #[cfg(windows)]
    #[test]
    fn header_matches_windows_bitmapv5header() {
        use windows::Win32::Graphics::Gdi::{BITMAPV5HEADER, BI_BITFIELDS as WIN_BITFIELDS};
        assert_eq!(std::mem::size_of::<BITMAPV5HEADER>(), DIBV5_HEADER_LEN);
        assert_eq!(WIN_BITFIELDS.0, BI_BITFIELDS);
        let mut dib = vec![0; dibv5_len(2, 3).unwrap()];
        write_dibv5(&mut dib, &sample(2, 3), 2, 3);
        // SAFETY: the buffer holds a full header; read_unaligned tolerates Vec alignment.
        let header = unsafe { dib.as_ptr().cast::<BITMAPV5HEADER>().read_unaligned() };
        let size = header.bV5Size;
        let (w, h) = (header.bV5Width, header.bV5Height);
        let (planes, bits) = (header.bV5Planes, header.bV5BitCount);
        let masks = (
            header.bV5RedMask,
            header.bV5GreenMask,
            header.bV5BlueMask,
            header.bV5AlphaMask,
        );
        assert_eq!((size, w, h, planes, bits), (124, 2, 3, 1, 32));
        assert_eq!(masks, (0xff0000, 0xff00, 0xff, 0xff000000));
        assert_eq!(header.bV5SizeImage, 2 * 3 * 4);
    }

    /// Exercises the exact global-memory payloads the clipboard would receive,
    /// without opening, emptying or writing the user's clipboard.
    #[cfg(windows)]
    #[test]
    fn prepared_global_payloads_round_trip_without_clipboard() {
        let (width, height) = (9, 4);
        let pixels = sample(width, height);
        let payload = win::prepare(&pixels, width, height).unwrap();
        let png = payload.png.with_bytes(|b| b.to_vec()).unwrap();
        let dib = payload.dib.with_bytes(|b| b.to_vec()).unwrap();
        assert_eq!(decode_png(&png), (width, height, pixels.clone()));
        assert_eq!(decode_dibv5(&dib), (width, height, pixels));
    }

    #[test]
    fn oversized_images_are_rejected_before_allocation() {
        assert_eq!(dibv5_len(1, 1), Some(DIBV5_HEADER_LEN + 4));
        assert_eq!(dibv5_len(1 << 31, 1), None);
        assert_eq!(dibv5_len(40_000, 40_000), None);
    }
}
