//! Production arrow renderer on authored pixels; no desktop capture or windows.
use snipping_tool_ul::{
    geometry::{Color, Point},
    icons::IconCache,
    tools::{render_annotation, Annotation},
    ui::Canvas,
};
use tiny_skia::Pixmap;
fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/arrow-proof.png".into());
    let mut p = Pixmap::new(940, 600).unwrap();
    p.fill(tiny_skia::Color::from_rgba8(23, 36, 42, 255));
    let mut icons = IconCache::new();
    let mut c = Canvas {
        pixmap: &mut p,
        scale: 1.,
        icons: &mut icons,
    };
    c.label(
        28.,
        20.,
        "Arrow shape · production renderer",
        24.,
        (230, 240, 241),
    );
    c.label(
        28.,
        59.,
        "Headless render on synthetic pixels",
        13.,
        (153, 179, 185),
    );
    for (i, t) in [3., 8., 20., 32.].into_iter().enumerate() {
        let y = 120. + i as f32 * 115.;
        c.label(28., y - 10., &format!("{t:.0} px"), 14., (183, 201, 207));
        for (start, end) in [
            (Point::new(130., y), Point::new(405., y)),
            (Point::new(510., y - 34.), Point::new(660., y + 34.)),
            (Point::new(870., y), Point::new(830., y)),
        ] {
            render_annotation(
                &Annotation::Arrow {
                    start,
                    end,
                    color: Color::new(83. / 255., 200. / 255., 214. / 255., 1.),
                    thickness: t,
                },
                c.pixmap,
                None,
                None,
            );
        }
    }
    p.save_png(&out).unwrap();
    println!("Headless arrow proof saved.");
}
