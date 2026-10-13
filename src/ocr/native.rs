use super::{text_result, Control, Stop};
use windows::{
    core::Interface,
    Foundation::AsyncStatus,
    Graphics::Imaging::{BitmapBufferAccessMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Win32::System::WinRT::{
        IMemoryBufferByteAccess, RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
    },
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
struct Bitmap {
    value: SoftwareBitmap,
    can_close: bool,
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        // The operation must reach a terminal state before closing its pixels.
        if self.can_close {
            let _ = self.value.Close();
        }
    }
}

fn win_error(error: windows::core::Error) -> String {
    format!("Windows OCR failed: {error}")
}

pub(super) fn recognize(
    pixels: &[u8],
    width: u32,
    height: u32,
    control: &Control,
) -> Result<String, String> {
    control.check()?;
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED).map_err(win_error)?;
    }
    let _apartment = Apartment;
    control.check()?;
    let max = OcrEngine::MaxImageDimension().map_err(win_error)?;
    if width > max || height > max {
        return Err(format!("Selected image exceeds Windows OCR's {max}-pixel dimension limit; select a smaller region"));
    }
    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|e| {
        if e.code().0 as u32 == 0x80004003 {
            "No OCR-capable language pack matches your Windows language preferences. Add one under Windows Settings > Time & Language > Language".into()
        } else { win_error(e) }
    })?;
    control.check()?;
    let mut bitmap = Bitmap {
        value: SoftwareBitmap::Create(BitmapPixelFormat::Gray8, width as i32, height as i32)
            .map_err(win_error)?,
        can_close: true,
    };
    {
        let buffer = bitmap
            .value
            .LockBuffer(BitmapBufferAccessMode::Write)
            .map_err(win_error)?;
        let plane = buffer.GetPlaneDescription(0).map_err(win_error)?;
        let reference = buffer.CreateReference().map_err(win_error)?;
        let access: IMemoryBufferByteAccess = reference.cast().map_err(win_error)?;
        let mut ptr = std::ptr::null_mut();
        let mut capacity = 0;
        unsafe {
            access
                .GetBuffer(&mut ptr, &mut capacity)
                .map_err(win_error)?;
        }
        let (start, stride) = layout(
            width,
            height,
            plane.StartIndex,
            plane.Stride,
            plane.Width,
            plane.Height,
            capacity as usize,
        )?;
        if ptr.is_null() {
            return Err("Invalid Windows OCR bitmap buffer".into());
        }
        // SAFETY: our live reference and exclusive write lock own this buffer.
        // Every addressed row is checked against the platform's capacity.
        let data = unsafe { std::slice::from_raw_parts_mut(ptr, capacity as usize) };
        for y in 0..height as usize {
            control.check()?;
            let offset = start + y * stride;
            for x in 0..width as usize {
                let p = &pixels[(y * width as usize + x) * 4..][..4];
                data[offset + x] = luminance(p);
            }
        }
        drop(access);
        reference.Close().map_err(win_error)?;
        buffer.Close().map_err(win_error)?;
    }
    control.check()?;
    let operation = engine.RecognizeAsync(&bitmap.value).map_err(win_error)?;
    #[cfg(test)]
    control
        .recognition_started
        .store(true, std::sync::atomic::Ordering::Release);
    bitmap.can_close = false;
    let terminal = wait_for_terminal(
        control,
        || operation.Status().map_err(win_error),
        || {
            let _ = operation.Cancel();
        },
    );
    bitmap.can_close = true;
    terminal?;
    let value = operation
        .GetResults()
        .map_err(win_error)?
        .Text()
        .map_err(win_error)?;
    text_result(value.as_wide())
}

pub(super) fn wait_for_terminal(
    control: &Control,
    mut status: impl FnMut() -> Result<AsyncStatus, String>,
    mut cancel: impl FnMut(),
) -> Result<(), String> {
    let mut stopped: Option<Stop> = None;
    loop {
        match status() {
            Ok(status) if status != AsyncStatus::Started => {
                control.check()?;
                if let Some(stop) = stopped {
                    return Err(stop.message().into());
                }
                return Ok(());
            }
            Ok(_) => {}
            Err(error) => {
                // Notify the caller promptly, but retain the worker's image and
                // single-flight lease while the OS operation's state is unknown.
                let _ = control.failure.set(error);
            }
        }
        if stopped.is_none() {
            if let Some(stop) = control
                .stop()
                .or_else(|| control.failure.get().map(|_| Stop::Cancelled))
            {
                // Cancellation is a request. Keep image ownership and the
                // single-flight lease until Windows reaches a terminal state.
                cancel();
                stopped = Some(stop);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(if stopped.is_some() {
            5
        } else {
            1
        }));
    }
}

fn layout(
    width: u32,
    height: u32,
    start: i32,
    stride: i32,
    plane_width: i32,
    plane_height: i32,
    capacity: usize,
) -> Result<(usize, usize), String> {
    if width == 0
        || height == 0
        || start < 0
        || stride < 0
        || plane_width < 0
        || plane_height < 0
        || (plane_width as u32) < width
        || (plane_height as u32) < height
        || (stride as usize) < width as usize
    {
        return Err("Invalid Windows OCR bitmap layout".into());
    }
    let start = start as usize;
    let stride = stride as usize;
    let end = (height as usize - 1)
        .checked_mul(stride)
        .and_then(|n| start.checked_add(n))
        .and_then(|n| n.checked_add(width as usize));
    if !end.is_some_and(|end| end <= capacity) {
        return Err("Invalid Windows OCR bitmap capacity".into());
    }
    Ok((start, stride))
}

fn luminance(p: &[u8]) -> u8 {
    let gray = (77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32 + 128) >> 8;
    let alpha = p[3] as u32;
    ((gray * alpha + 255 * (255 - alpha) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout_honors_stride_padding_and_rejects_short_or_invalid_buffers() {
        assert_eq!(layout(3, 2, 4, 8, 3, 2, 15).unwrap(), (4, 8));
        for args in [
            (3, 2, 4, 8, 3, 2, 14),
            (3, 2, -1, 8, 3, 2, 20),
            (3, 2, 0, -8, 3, 2, 20),
            (3, 2, 0, 2, 3, 2, 20),
            (3, 2, 0, 8, 2, 2, 20),
            (3, 2, 0, 8, 3, 1, 20),
            (0, 2, 0, 8, 3, 2, 20),
        ] {
            assert!(layout(args.0, args.1, args.2, args.3, args.4, args.5, args.6).is_err());
        }
    }
    #[test]
    fn grayscale_preserves_black_white_and_composites_hidden_color_over_white() {
        assert_eq!(luminance(&[0, 0, 0, 255]), 0);
        assert_eq!(luminance(&[255, 255, 255, 255]), 255);
        assert_eq!(luminance(&[255, 0, 0, 0]), 255);
        assert_eq!(luminance(&[0, 0, 0, 128]), 127);
        assert_eq!(luminance(&[255, 0, 0, 255]), 77);
        assert_eq!(luminance(&[0, 255, 0, 255]), 149);
        assert_eq!(luminance(&[0, 0, 255, 255]), 29);
    }
}
