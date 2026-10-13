use snipping_tool_ul::{
    geometry::{Color, Point},
    tools::{annotation_bounding_box, hit_test_annotation, render_annotation, Annotation},
};
use tiny_skia::Pixmap;
fn arrow(start: Point, end: Point, width: f32, alpha: f32) -> Annotation {
    Annotation::Arrow {
        start,
        end,
        thickness: width,
        color: Color::new(1., 0., 0., alpha),
    }
}
#[test]
fn thick_tips_taper_in_every_direction() {
    for t in [8., 16., 32., 64.] {
        for (ux, uy) in [
            (1., 0.),
            (-1., 0.),
            (0., 1.),
            (0., -1.),
            (0.6, 0.8),
            (-0.6, 0.8),
        ] {
            let end = Point::new(320., 320.);
            let start = Point::new(end.x - ux * 290., end.y - uy * 290.);
            let mut p = Pixmap::new(640, 640).unwrap();
            render_annotation(&arrow(start, end, t, 1.), &mut p, None, None);
            let x = (end.x - ux - uy * t * 0.35).floor() as u32;
            let y = (end.y - uy + ux * t * 0.35).floor() as u32;
            assert_eq!(
                p.pixel(x, y).unwrap().alpha(),
                0,
                "shaft protrudes beside tip: {t} ({ux},{uy})"
            );
        }
    }
}
#[test]
fn translucent_arrow_has_no_overlap_seam() {
    let mut p = Pixmap::new(320, 160).unwrap();
    render_annotation(
        &arrow(Point::new(30., 80.), Point::new(290., 80.), 20., 0.5),
        &mut p,
        None,
        None,
    );
    assert!(p.pixels().iter().any(|x| x.alpha() == 128));
    assert!(
        p.pixels().iter().all(|x| x.alpha() <= 128),
        "shaft/head overlap darkens translucent arrows"
    );
}
#[test]
fn bounds_contain_arrowheads_and_short_arrows() {
    for t in [3., 8., 20., 32.] {
        for length in [2., 12., 50., 200.] {
            for (ux, uy) in [(1., 0.), (-1., 0.), (0., 1.), (0., -1.), (0.6, 0.8)] {
                let start = Point::new(260., 260.);
                let end = Point::new(start.x + ux * length, start.y + uy * length);
                let a = arrow(start, end, t, 1.);
                let (x, y, w, h) = annotation_bounding_box(&a).unwrap();
                let mut p = Pixmap::new(520, 520).unwrap();
                render_annotation(&a, &mut p, None, None);
                for (i, _v) in p.pixels().iter().enumerate().filter(|(_, v)| v.alpha() > 0) {
                    let px = (i % 520) as f32 + 0.5;
                    let py = (i / 520) as f32 + 0.5;
                    assert!(
                        px >= x - 1. && px <= x + w + 1. && py >= y - 1. && py <= y + h + 1.,
                        "visible arrow outside selection bounds: {t} {length} {px},{py}"
                    );
                }
                assert!(
                    x >= start.x.min(end.x) - t * 3.
                        && w <= length + 6. * t
                        && h <= length + 6. * t
                );
            }
        }
    }
}
#[test]
fn zero_length_and_invalid_arrows_draw_nothing() {
    for (start, end, t) in [
        (Point::new(50., 50.), Point::new(50., 50.), 10.),
        (Point::new(f32::NAN, 20.), Point::new(60., 50.), 10.),
        (Point::new(20., 20.), Point::new(60., 50.), 0.),
    ] {
        let a = arrow(start, end, t, 1.);
        let mut p = Pixmap::new(100, 100).unwrap();
        render_annotation(&a, &mut p, None, None);
        assert!(p.pixels().iter().all(|x| x.alpha() == 0));
        assert!(annotation_bounding_box(&a).is_none());
    }
}
#[test]
fn arrowhead_wings_can_be_selected() {
    let a = arrow(Point::new(10., 80.), Point::new(210., 80.), 10., 1.);
    assert!(hit_test_annotation(&a, &Point::new(175., 100.), 1.));
    assert!(!hit_test_annotation(&a, &Point::new(205., 110.), 1.));
}
