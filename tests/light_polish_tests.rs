//! Hide private info, clipboard state and drag handle: headless checks.
use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editor_ui::EditorUi,
    geometry::{Point, Size},
    launcher::Action,
    state::OverlayState,
    tools::Annotation,
};

fn state() -> OverlayState {
    OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 64 * 64 * 4],
            width: 64,
            height: 64,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    )
}

fn pixelate(x: f32) -> Annotation {
    Annotation::Pixelate {
        top_left: Point::new(x, 1.),
        size: Size::new(5., 5.),
        block_size: 10,
    }
}

#[test]
fn hidden_details_undo_and_redo_as_one_step() {
    let mut s = state();
    s.annotations.push(pixelate(0.));
    s.add_annotations(vec![pixelate(10.), pixelate(20.), pixelate(30.)]);
    assert_eq!(s.annotations.len(), 4);
    assert!(s.undo_annotation());
    assert_eq!(s.annotations, vec![pixelate(0.)]);
    assert!(s.redo_annotation());
    assert_eq!(
        s.annotations,
        vec![pixelate(0.), pixelate(10.), pixelate(20.), pixelate(30.)]
    );
    // Nothing found records nothing.
    let undo = s.undo_stack.len();
    s.add_annotations(Vec::new());
    assert_eq!(s.undo_stack.len(), undo);
}

#[test]
fn copy_state_follows_edits_text_and_other_apps() {
    let mut s = state();
    assert!(!s.clipboard_current(7), "never copied");
    s.clipboard_copy = Some((Vec::new(), 7));
    assert!(s.clipboard_current(7));
    assert!(!s.clipboard_current(8), "another app copied since");
    s.annotations.push(pixelate(0.));
    assert!(!s.clipboard_current(7), "edited since the copy");
    s.clipboard_copy = Some((s.annotations.clone(), 9));
    assert!(s.clipboard_current(9));
    s.text_input_active = true;
    s.text_input_buffer = "typing".into();
    assert!(!s.clipboard_current(9), "unfinished text is a change");
}

#[test]
fn size_label_is_the_drag_handle_at_every_supported_size() {
    let s = state();
    let ui = EditorUi::new(&Config::default());
    for (w, h) in [(660., 390.), (940., 612.), (1920., 1080.)] {
        assert_eq!(
            ui.hit(Point::new(w - 50., h - 22.), w, h, &s),
            Some(Action::DragOut),
            "{w}x{h}"
        );
    }
}
