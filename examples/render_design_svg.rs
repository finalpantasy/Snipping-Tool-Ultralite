//! Offline design-study renderer. No native windows, clipboard or capture.
fn main() {
    let mut opts = resvg::usvg::Options::default();
    // Studies use the app's only typeface.
    opts.fontdb_mut()
        .load_font_data(include_bytes!("../assets/font.ttf").to_vec());
    opts.font_family = "Open Sans".into();
    for filename in std::env::args().skip(1) {
        let path = std::path::PathBuf::from(filename);
        let svg = std::fs::read_to_string(&path).expect("SVG source");
        let tree = resvg::usvg::Tree::from_str(&svg, &opts).expect("SVG parse");
        let size = tree.size().to_int_size();
        let mut p = tiny_skia::Pixmap::new(size.width(), size.height()).expect("Pixmap");
        resvg::render(&tree, tiny_skia::Transform::identity(), &mut p.as_mut());
        p.save_png(path.with_extension("png")).expect("PNG save");
    }
    println!("Concept previews rendered headlessly.");
}
