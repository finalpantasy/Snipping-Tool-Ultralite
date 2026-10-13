//! Alpha-correct HWND presentation with a bounded opaque software fallback.
use std::{num::NonZeroU32, sync::Arc};
use winit::window::Window;
pub struct Surface {
    // Backend resources must drop before the last HWND owner.
    backend: Backend,
    window: Arc<Window>,
}
enum Backend {
    Soft(softbuffer::Surface<Arc<Window>, Arc<Window>>),
    #[cfg(target_os = "windows")]
    Glass(Glass),
}
impl Surface {
    pub fn uses_glass(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            matches!(self.backend, Backend::Glass(_))
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
    pub fn new(window: Arc<Window>) -> Result<Self, String> {
        crate::launcher::style_window(&window);
        #[cfg(target_os = "windows")]
        if let Ok(glass) = Glass::new(&window) {
            crate::launcher::style_window(&window);
            return Ok(Self {
                window,
                backend: Backend::Glass(glass),
            });
        }
        disable_glass(&window);
        let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
        let backend = Backend::Soft(
            softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?,
        );
        Ok(Self { window, backend })
    }
    pub fn present(&mut self, p: &tiny_skia::Pixmap) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        if let Backend::Glass(g) = &mut self.backend {
            match g.present(p) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("Glass presentation failed: {e}");
                }
            }
            // A display change, RDP session switch or GPU reset returns
            // D2DERR_RECREATE_TARGET; one rebuild keeps the glass look instead
            // of downgrading this window to opaque for the rest of its life.
            if let Ok(rebuilt) = Glass::new(&self.window) {
                self.backend = Backend::Glass(rebuilt);
                if let Backend::Glass(g) = &mut self.backend {
                    if g.present(p).is_ok() {
                        return Ok(());
                    }
                }
            }
            tracing::warn!("Glass presentation unavailable; using opaque fallback");
            self.fall_back_to_soft()?;
        }
        self.present_soft(p.data(), p.width(), p.height())
    }

    /// Present a frame the caller fully redraws before the next present. The
    /// pixel data is swizzled in place for the glass path, so no second
    /// full-frame buffer is kept (5 MB at the default window, 33 MB at 4K).
    pub fn present_consumed(&mut self, p: &mut tiny_skia::Pixmap) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        if let Backend::Glass(g) = &mut self.backend {
            match g.present_in_place(p) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("Glass presentation failed: {e}");
                }
            }
            if let Ok(rebuilt) = Glass::new(&self.window) {
                self.backend = Backend::Glass(rebuilt);
                if let Backend::Glass(g) = &mut self.backend {
                    if g.present_in_place(p).is_ok() {
                        return Ok(());
                    }
                }
            }
            tracing::warn!("Glass presentation unavailable; using opaque fallback");
            self.fall_back_to_soft()?;
        }
        self.present_soft(p.data(), p.width(), p.height())
    }

    #[cfg(target_os = "windows")]
    fn fall_back_to_soft(&mut self) -> Result<(), String> {
        {
            disable_glass(&self.window);
            let context =
                softbuffer::Context::new(self.window.clone()).map_err(|e| e.to_string())?;
            self.backend = Backend::Soft(
                softbuffer::Surface::new(&context, self.window.clone())
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(())
    }

    fn present_soft(&mut self, data: &[u8], width: u32, height: u32) -> Result<(), String> {
        let Backend::Soft(surface) = &mut self.backend else {
            return Err("Presenter unavailable".into());
        };
        let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return Ok(());
        };
        surface.resize(w, h).map_err(|e| e.to_string())?;
        let mut buffer = surface.buffer_mut().map_err(|e| e.to_string())?;
        // No extra image allocation in the fallback path.
        let rgb = crate::ui::base();
        for (dst, px) in buffer.iter_mut().zip(data.chunks_exact(4)) {
            let inv = 255 - px[3] as u32;
            let c = |i: usize, b: u8| px[i] as u32 + (b as u32 * inv + 127) / 255;
            *dst = c(0, rgb.0) << 16 | c(1, rgb.1) << 8 | c(2, rgb.2);
        }
        buffer.present().map_err(|e| e.to_string())
    }
}
#[cfg(target_os = "windows")]
fn hwnd(w: &Window) -> Result<windows::Win32::Foundation::HWND, String> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match w.window_handle().map_err(|e| e.to_string())?.as_raw() {
        RawWindowHandle::Win32(h) => Ok(windows::Win32::Foundation::HWND(h.hwnd.get() as _)),
        _ => Err("Windows HWND required".into()),
    }
}
fn disable_glass(window: &Window) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::{Graphics::Dwm::*, UI::Controls::MARGINS};
        if let Ok(hwnd) = hwnd(window) {
            let none = 1u32;
            let _ =
                DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, &none as *const _ as _, 4);
            let _ = DwmExtendFrameIntoClientArea(hwnd, &MARGINS::default());
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = window;
    crate::launcher::style_window(window);
}
#[cfg(target_os = "windows")]
struct Glass {
    target: windows::Win32::Graphics::Direct2D::ID2D1HwndRenderTarget,
    bitmap: Option<windows::Win32::Graphics::Direct2D::ID2D1Bitmap>,
    size: (u32, u32),
    bgra: Vec<u8>,
}
#[cfg(target_os = "windows")]
impl Glass {
    fn new(window: &Window) -> Result<Self, String> {
        unsafe {
            use windows::Win32::{
                Graphics::{Direct2D::Common::*, Direct2D::*, Dwm::*, Dxgi::Common::*},
                System::Registry::*,
                UI::{Accessibility::*, Controls::MARGINS, WindowsAndMessaging::*},
            };
            let mut high = HIGHCONTRASTW {
                cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
                ..Default::default()
            };
            if SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                high.cbSize,
                Some(&mut high as *mut _ as _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
            .is_ok()
                && high.dwFlags & HCF_HIGHCONTRASTON != HIGHCONTRASTW_FLAGS(0)
            {
                return Err("High contrast uses solid surfaces".into());
            }
            let mut transparency = 1u32;
            let mut bytes = 4u32;
            let _ = RegGetValueW(
                HKEY_CURRENT_USER,
                windows::core::w!(
                    "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
                ),
                windows::core::w!("EnableTransparency"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut transparency as *mut _ as _),
                Some(&mut bytes),
            );
            if transparency == 0 {
                return Err("Transparency effects disabled".into());
            }
            let hwnd = hwnd(window)?;
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
                .map_err(|e| e.to_string())?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                // CPU-rendered UI does not need a per-window GPU device/driver cache.
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.,
                dpiY: 96.,
                ..Default::default()
            };
            let size = window.inner_size();
            let target = factory
                .CreateHwndRenderTarget(
                    &props,
                    &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                        hwnd,
                        pixelSize: D2D_SIZE_U {
                            width: size.width.max(1),
                            height: size.height.max(1),
                        },
                        presentOptions: D2D1_PRESENT_OPTIONS_IMMEDIATELY,
                    },
                )
                .map_err(|e| e.to_string())?;
            let backdrop = 3u32;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop as *const _ as _,
                4,
            )
            .map_err(|e| e.to_string())?;
            DwmExtendFrameIntoClientArea(
                hwnd,
                &MARGINS {
                    cxLeftWidth: -1,
                    cxRightWidth: -1,
                    cyTopHeight: -1,
                    cyBottomHeight: -1,
                },
            )
            .map_err(|e| e.to_string())?;
            Ok(Self {
                target,
                bitmap: None,
                size: (0, 0),
                bgra: Vec::new(),
            })
        }
    }
    fn present(&mut self, p: &tiny_skia::Pixmap) -> Result<(), String> {
        let size = (p.width(), p.height());
        if size != self.size && self.bgra.capacity() > p.data().len().saturating_mul(2) {
            self.bgra = Vec::new();
        }
        self.bgra.resize(p.data().len(), 0);
        for (dst, src) in self.bgra.chunks_exact_mut(4).zip(p.data().chunks_exact(4)) {
            dst.copy_from_slice(&[src[2], src[1], src[0], src[3]]);
        }
        let bgra = std::mem::take(&mut self.bgra);
        let result = unsafe { self.upload_and_draw(&bgra, size) };
        self.bgra = bgra;
        result
    }

    /// Swizzle the frame to BGRA in place and present it; the caller redraws
    /// the whole pixmap before presenting again. Restores RGBA on failure so
    /// the opaque fallback still receives correct data.
    fn present_in_place(&mut self, p: &mut tiny_skia::Pixmap) -> Result<(), String> {
        let size = (p.width(), p.height());
        // Drop the retained buffer from any earlier `present` call.
        self.bgra = Vec::new();
        let swizzle = |data: &mut [u8]| {
            for px in data.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        };
        swizzle(p.data_mut());
        let result = unsafe { self.upload_and_draw(p.data(), size) };
        if result.is_err() {
            swizzle(p.data_mut());
        }
        result
    }

    unsafe fn upload_and_draw(&mut self, bgra: &[u8], size: (u32, u32)) -> Result<(), String> {
        unsafe {
            use windows::Win32::Graphics::{Direct2D::Common::*, Direct2D::*, Dxgi::Common::*};
            if size != self.size {
                self.target
                    .Resize(&D2D_SIZE_U {
                        width: size.0,
                        height: size.1,
                    })
                    .map_err(|e| e.to_string())?;
                self.bitmap = Some(
                    self.target
                        .CreateBitmap(
                            D2D_SIZE_U {
                                width: size.0,
                                height: size.1,
                            },
                            Some(bgra.as_ptr() as _),
                            size.0 * 4,
                            &D2D1_BITMAP_PROPERTIES {
                                pixelFormat: D2D1_PIXEL_FORMAT {
                                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                                },
                                dpiX: 96.,
                                dpiY: 96.,
                            },
                        )
                        .map_err(|e| e.to_string())?,
                );
                self.size = size;
            } else if let Some(bitmap) = &self.bitmap {
                bitmap
                    .CopyFromMemory(None, bgra.as_ptr() as _, size.0 * 4)
                    .map_err(|e| e.to_string())?;
            }
            self.target.BeginDraw();
            self.target.Clear(None);
            self.target.DrawBitmap(
                self.bitmap.as_ref().unwrap(),
                None,
                1.,
                D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
                None,
            );
            self.target.EndDraw(None, None).map_err(|e| e.to_string())
        }
    }
}
