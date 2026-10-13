//! Synthetic native presentation probe. Refuses the normal desktop before any GUI call.
#[cfg(target_os = "windows")]
fn main() {
    use std::{sync::Arc, time::Instant};
    use windows::Win32::{
        Foundation::HANDLE,
        System::{ProcessStatus::*, StationsAndDesktops::*, Threading::*},
    };
    use winit::{
        application::ApplicationHandler,
        event_loop::{ActiveEventLoop, EventLoop},
        window::WindowAttributes,
    };
    fn desktop_name() -> String {
        unsafe {
            let desk = GetThreadDesktop(GetCurrentThreadId()).expect("Thread desktop");
            let mut name = [0u16; 256];
            GetUserObjectInformationW(
                HANDLE(desk.0),
                UOI_NAME,
                Some(name.as_mut_ptr() as _),
                512,
                None,
            )
            .expect("Desktop name");
            String::from_utf16_lossy(
                &name[..name.iter().position(|c| *c == 0).unwrap_or(name.len())],
            )
        }
    }
    fn memory() -> usize {
        unsafe {
            let mut counters = PROCESS_MEMORY_COUNTERS {
                cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                ..Default::default()
            };
            let bytes = counters.cb;
            GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, bytes)
                .expect("Own process memory");
            counters.WorkingSetSize
        }
    }
    let name = desktop_name();
    if !name.starts_with("SnippingToolUL.Test.") {
        eprintln!("Refusing native probe outside a private test desktop.");
        std::process::exit(2);
    }
    let output = std::env::args().nth(1).expect("Output report path");
    snipping_tool_ul::capture::initialize_dpi();
    struct Probe {
        rows: Vec<serde_json::Value>,
    }
    impl ApplicationHandler for Probe {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let cfg = snipping_tool_ul::config::Config::default();
            let model = snipping_tool_ul::launcher::Model::new(
                cfg.workflow.clone(),
                cfg.hotkey.capture.clone(),
            );
            for cycle in 0..10 {
                snipping_tool_ul::theme::set_mode(if cycle % 2 == 0 {
                    snipping_tool_ul::theme::ThemeMode::Dark
                } else {
                    snipping_tool_ul::theme::ThemeMode::Light
                });
                let window = Arc::new(
                    event_loop
                        .create_window(
                            WindowAttributes::default()
                                .with_title("Snipping Tool UL isolated probe")
                                .with_inner_size(winit::dpi::LogicalSize::new(660, 372))
                                .with_visible(false),
                        )
                        .expect("Private window"),
                );
                let mut surface = snipping_tool_ul::native_surface::Surface::new(window.clone())
                    .expect("Native presenter");
                // These DWM color attributes support setters, not read-back queries.
                let caption_theme_applied = snipping_tool_ul::launcher::style_window(&window);
                assert!(
                    caption_theme_applied,
                    "Windows accepts caption theme settings"
                );
                let size = window.inner_size();
                let mut p =
                    tiny_skia::Pixmap::new(size.width, size.height).expect("UI framebuffer");
                model.render(&mut p);
                let start = Instant::now();
                surface.present(&p).expect("Present synthetic UI");
                window.set_visible(true);
                self.rows.push(serde_json::json!({"cycle":cycle,"theme":if cycle % 2 == 0 {"dark"} else {"light"},"caption_theme_applied":caption_theme_applied,"glass_backend":surface.uses_glass(),"present_ms":start.elapsed().as_secs_f64()*1000.,"working_set_bytes":memory(),"frame_bytes":p.data().len()}));
                window.set_visible(false);
                drop(surface);
                drop(p);
                drop(window);
                self.rows
                    .push(serde_json::json!({"after_close":cycle,"working_set_bytes":memory()}));
            }
            event_loop.exit();
        }
        fn window_event(
            &mut self,
            _: &ActiveEventLoop,
            _: winit::window::WindowId,
            _: winit::event::WindowEvent,
        ) {
        }
    }
    let baseline = memory();
    let mut app = Probe { rows: Vec::new() };
    EventLoop::new()
        .expect("Private event loop")
        .run_app(&mut app)
        .expect("Private probe loop");
    std::fs::write(output,serde_json::to_vec_pretty(&serde_json::json!({"desktop":name,"baseline_bytes":baseline,"samples":app.rows,"capture_or_clipboard_operations":0,"visual_composition_verified":false})).unwrap()).expect("Probe report");
}
#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Windows-only private UI probe.");
    std::process::exit(2);
}
