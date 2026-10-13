use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    geometry::{Color, Point},
    state::OverlayState,
    tools::{Annotation, ResizeHandle},
};

fn state() -> OverlayState {
    let mut s = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 64 * 64 * 4],
            width: 64,
            height: 64,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    s.annotations = vec![
        Annotation::Line {
            start: Point::new(1., 2.),
            end: Point::new(10., 20.),
            color: Color::red(),
            thickness: 3.,
        },
        Annotation::Line {
            start: Point::new(30., 40.),
            end: Point::new(50., 60.),
            color: Color::blue(),
            thickness: 8.,
        },
    ];
    s
}
fn start_drag(s: &mut OverlayState) {
    s.selected_index = Some(0);
    s.pre_drag_annotation = Some((0, s.annotations[0].clone()));
    s.select_drag_start = Some(Point::new(1., 2.));
    snipping_tool_ul::tools::move_annotation(&mut s.annotations[0], 7., 8.);
}

#[test]
fn deleting_while_dragging_never_records_another_marks_snapshot() {
    let mut s = state();
    let initial = s.annotations.clone();
    start_drag(&mut s);
    let moved = s.annotations.clone();
    assert!(s.delete_annotation(0));
    assert!(s.pre_drag_annotation.is_none());
    assert!(s.select_drag_start.is_none());
    assert!(s.resize_handle.is_none());
    s.finish_annotation_drag(); // Mouse release after Delete.
    assert_eq!(s.annotations, vec![initial[1].clone()]);
    assert!(s.undo_annotation());
    assert_eq!(s.annotations, moved);
    assert!(s.undo_annotation());
    assert_eq!(s.annotations, initial);
    assert!(s.redo_annotation());
    assert!(s.redo_annotation());
    assert_eq!(s.annotations, vec![initial[1].clone()]);
}

#[test]
fn undo_during_drag_and_later_release_do_not_modify_neighbors() {
    let mut s = state();
    let initial = s.annotations.clone();
    start_drag(&mut s);
    assert!(s.undo_annotation());
    s.finish_annotation_drag();
    assert_eq!(s.annotations, initial);
    assert!(s.undo_stack.is_empty());
    assert!(s.redo_annotation());
    assert_ne!(s.annotations[0], initial[0]);
    assert_eq!(s.annotations[1], initial[1]);
}

#[test]
fn escape_restores_uncommitted_drag_and_clears_resize_state() {
    let mut s = state();
    let initial = s.annotations.clone();
    start_drag(&mut s);
    s.resize_handle = Some(ResizeHandle::BottomRight);
    s.cancel_annotation_drag();
    s.finish_annotation_drag();
    assert_eq!(s.annotations, initial);
    assert!(s.undo_stack.is_empty());
    assert!(s.resize_handle.is_none());
}
