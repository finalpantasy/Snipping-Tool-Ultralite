//! Offline preparation of authored app/tray resources. No windows or desktop access.
use std::path::Path;
fn preserve_or_create(path: &Path, data: &[u8]) {
    if path.exists() {
        assert_eq!(
            std::fs::read(path).unwrap(),
            data,
            "Preserve existing asset"
        );
    } else {
        std::fs::write(path, data).unwrap();
    }
}
fn bird(tree: &resvg::usvg::Tree, size: u32) -> tiny_skia::Pixmap {
    let mut p = tiny_skia::Pixmap::new(size, size).unwrap();
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut p.as_mut(),
    );
    p
}
fn windows_ico(frames: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut ico = vec![0, 0, 1, 0];
    ico.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    let mut offset = 6 + frames.len() * 16;
    for (size, png) in frames {
        let dimension = if *size == 256 { 0 } else { *size as u8 };
        ico.extend_from_slice(&[dimension, dimension, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in frames {
        ico.extend_from_slice(png);
    }
    ico
}
fn tray(svg: &str, color: &str) -> tiny_skia::Pixmap {
    let svg = svg.replace("currentColor", color);
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    assert_eq!(tree.size().to_int_size().width(), 32);
    assert_eq!(tree.size().to_int_size().height(), 32);
    let mut p = tiny_skia::Pixmap::new(32, 32).unwrap();
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut p.as_mut());
    p
}
fn straight_rgba(p: &tiny_skia::Pixmap) -> Vec<u8> {
    let mut data = p.data().to_vec();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    data
}
fn main() {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/logo-assets".into());
    let out = Path::new(&output);
    std::fs::create_dir_all(out).unwrap();
    let source = std::fs::read_to_string("assets/ul-logo.svg").unwrap();
    let tree = resvg::usvg::Tree::from_str(&source, &resvg::usvg::Options::default()).unwrap();
    let app_pixels = bird(&tree, 64);
    let app = image::RgbaImage::from_raw(64, 64, straight_rgba(&app_pixels)).unwrap();
    let frames: Vec<_> = [16, 20, 24, 32, 40, 48, 64, 128, 256]
        .into_iter()
        .map(|size| (size, bird(&tree, size).encode_png().unwrap()))
        .collect();
    preserve_or_create(&out.join("ul-icon.ico"), &windows_ico(&frames));
    preserve_or_create(&out.join("ul-icon.rgba"), app.as_raw());
    preserve_or_create(&out.join("ul-icon.png"), &frames.last().unwrap().1);
    let source = std::fs::read_to_string("assets/ul-tray.svg").unwrap();
    // Teal in both taskbar modes; brighter ink survives dark shells and deeper ink light shells.
    let dark = tray(&source, "#53c8d6");
    let light = tray(&source, "#007c91");
    for (name, p) in [("dark", &dark), ("light", &light)] {
        preserve_or_create(
            &out.join(format!("ul-tray-on-{name}.rgba")),
            &straight_rgba(p),
        );
        preserve_or_create(
            &out.join(format!("ul-tray-on-{name}.png")),
            &p.encode_png().unwrap(),
        );
    }
    let mut proof = image::RgbaImage::new(480, 128);
    for (x, _, px) in proof.enumerate_pixels_mut() {
        *px = if x < 240 {
            image::Rgba([236, 244, 243, 255])
        } else {
            image::Rgba([20, 32, 37, 255])
        };
    }
    for (x, icon) in [(0, &light), (240, &dark)] {
        image::imageops::overlay(&mut proof, &app, x + 20, 32);
        let small_app = image::load_from_memory(&frames[0].1).unwrap().into_rgba8();
        image::imageops::overlay(&mut proof, &small_app, x + 86, 56);
        let pixels = image::RgbaImage::from_raw(32, 32, straight_rgba(icon)).unwrap();
        image::imageops::overlay(&mut proof, &pixels, x + 110, 48);
        let small = image::imageops::resize(&pixels, 16, 16, image::imageops::FilterType::Lanczos3);
        image::imageops::overlay(&mut proof, &small, x + 174, 56);
    }
    proof.save(out.join("icon-proof.png")).unwrap();
    println!("Prepared transparent teal app icons at nine sizes and adaptive teal tray pixels.");
}
