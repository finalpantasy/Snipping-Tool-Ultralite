use snipping_tool_ul::{
    capture::CapturedScreen,
    config::Config,
    editing::SizeKind,
    geometry::{Color, Point},
    state::OverlayState,
    tools::{Annotation, ToolKind, UndoAction},
};
#[test]
fn unchanged_text_reedit_keeps_redo_and_exposes_active_size() {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 8 * 8 * 4],
            width: 8,
            height: 8,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    let old = Annotation::Text {
        position: Point::new(1., 2.),
        text: "Caption".into(),
        color: Color::red(),
        font_size: 20.,
    };
    o.redo_stack.push(UndoAction::Add(0));
    o.text_edit_origin = Some((0, old.clone()));
    o.text_input_active = true;
    o.text_input_buffer = "Caption".into();
    o.text_input_position = Point::new(1., 2.);
    o.text_input_font_size = 20.;
    o.current_color = Color::red();
    o.active_tool = ToolKind::Select;
    assert_eq!(o.size_kind(), SizeKind::Text);
    assert_eq!(o.tool_size(), 20.);
    o.commit_text();
    assert_eq!(o.annotations, vec![old]);
    assert!(o.undo_stack.is_empty());
    assert_eq!(o.redo_stack.len(), 1);
}

#[test]
fn right_click_during_text_reedit_selects_the_clicked_annotation() {
    let mut o = OverlayState::new(
        CapturedScreen {
            pixels: vec![255; 200 * 100 * 4],
            width: 200,
            height: 100,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.,
        },
        &Config::default(),
    );
    let rect = |x: f32| Annotation::Arrow {
        start: Point::new(x, 10.),
        end: Point::new(x + 20., 30.),
        color: Color::red(),
        thickness: 3.,
    };
    let text = Annotation::Text {
        position: Point::new(5., 60.),
        text: "Caption".into(),
        color: Color::red(),
        font_size: 20.,
    };
    // [T, A, B] with T removed for re-editing leaves [A, B].
    o.annotations = vec![rect(10.), rect(100.)];
    o.text_edit_origin = Some((0, text.clone()));
    o.text_input_active = true;
    o.text_input_buffer = "Caption 2".into();
    o.text_input_position = Point::new(5., 60.);
    o.text_input_font_size = 20.;
    let hit = o.select_for_context_menu(&Point::new(110., 20.), 4.);
    // The text is back at index 0, so B is index 2 and still the selection.
    assert_eq!(hit, Some(2));
    assert_eq!(o.annotations[2], rect(100.));
    assert!(!o.text_input_active);
}
