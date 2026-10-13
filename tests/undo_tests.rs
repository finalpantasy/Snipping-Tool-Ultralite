use snipping_tool_ul::geometry::{Color, Point};
use snipping_tool_ul::tools::{
    apply_redo, apply_undo, move_annotation, record_undo, Annotation, UndoAction,
};

fn make_arrow() -> Annotation {
    Annotation::Arrow {
        start: Point::new(0.0, 0.0),
        end: Point::new(100.0, 100.0),
        color: Color::red(),
        thickness: 3.0,
    }
}

#[test]
fn test_undo_add() {
    let mut annotations = vec![];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    // Simulate adding an annotation
    let ann = make_arrow();
    annotations.push(ann);
    record_undo(&mut undo_stack, &mut redo_stack, UndoAction::Add(0));

    assert_eq!(annotations.len(), 1);

    // Undo should remove it
    let result = apply_undo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(result);
    assert_eq!(annotations.len(), 0);

    // Redo should restore it
    let result = apply_redo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(result);
    assert_eq!(annotations.len(), 1);
}

#[test]
fn test_undo_delete() {
    let ann = make_arrow();
    let mut annotations = vec![ann.clone()];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    // Simulate delete
    let removed = annotations.remove(0);
    record_undo(
        &mut undo_stack,
        &mut redo_stack,
        UndoAction::Delete(0, removed),
    );
    assert_eq!(annotations.len(), 0);

    // Undo should restore
    let result = apply_undo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(result);
    assert_eq!(annotations.len(), 1);

    // Redo should delete again
    let result = apply_redo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(result);
    assert_eq!(annotations.len(), 0);
}

#[test]
fn test_undo_modify() {
    let ann = make_arrow();
    let mut annotations = vec![ann.clone()];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    // Simulate modify (move)
    let old = annotations[0].clone();
    move_annotation(&mut annotations[0], 50.0, 50.0);
    record_undo(&mut undo_stack, &mut redo_stack, UndoAction::Modify(0, old));

    // Verify moved
    match &annotations[0] {
        Annotation::Arrow { start, .. } => assert_eq!(start.x, 50.0),
        _ => panic!("Expected Arrow"),
    }

    // Undo should restore original position
    apply_undo(&mut annotations, &mut undo_stack, &mut redo_stack);
    match &annotations[0] {
        Annotation::Arrow { start, .. } => assert_eq!(start.x, 0.0),
        _ => panic!("Expected Arrow"),
    }

    // Redo should re-apply the move
    apply_redo(&mut annotations, &mut undo_stack, &mut redo_stack);
    match &annotations[0] {
        Annotation::Arrow { start, .. } => assert_eq!(start.x, 50.0),
        _ => panic!("Expected Arrow"),
    }
}

#[test]
fn test_undo_stack_cap() {
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    // Push 55 actions; cap is 50
    for i in 0..55 {
        record_undo(&mut undo_stack, &mut redo_stack, UndoAction::Add(i));
    }
    assert_eq!(undo_stack.len(), 50);
}

#[test]
fn test_new_action_clears_redo() {
    let ann = make_arrow();
    let mut annotations = vec![ann];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    // Record an add, then undo to populate redo
    record_undo(&mut undo_stack, &mut redo_stack, UndoAction::Add(0));
    apply_undo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert_eq!(redo_stack.len(), 1);

    // A new action should clear redo
    annotations.push(make_arrow());
    record_undo(&mut undo_stack, &mut redo_stack, UndoAction::Add(0));
    assert_eq!(redo_stack.len(), 0);
}

#[test]
fn test_undo_empty_stack_returns_false() {
    let mut annotations = vec![];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    let result = apply_undo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(!result);
}

#[test]
fn test_redo_empty_stack_returns_false() {
    let mut annotations = vec![];
    let mut undo_stack = vec![];
    let mut redo_stack = vec![];

    let result = apply_redo(&mut annotations, &mut undo_stack, &mut redo_stack);
    assert!(!result);
}
