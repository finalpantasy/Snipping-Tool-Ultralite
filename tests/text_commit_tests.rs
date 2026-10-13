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
