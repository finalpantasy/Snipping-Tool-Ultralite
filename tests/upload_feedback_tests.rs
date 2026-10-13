//! Production presentation state only: no HTTP, clipboard, native windows or hotkeys.
use snipping_tool_ul::{
    config::Config,
    geometry::Point,
    launcher::{Action, Menu, Model, COMPACT_HEIGHT, WIDTH},
    upload_feedback::UploadFeedback as F,
};

#[test]
fn uploaded_does_not_claim_copied_when_clipboard_fails_or_is_newer() {
    for outcome in [Ok(false), Err(())] {
        let f = F::completed("https://files.catbox.moe/test.png".into(), outcome, false);
        assert!(matches!(f, F::Uploaded { .. }));
        assert_eq!(f.button_label(), "Copy link");
        assert!(f.heading().contains("not copied"));
        assert!(f.detail().starts_with("https://"));
    }
    for reused in [false, true] {
        let f = F::completed("https://files.catbox.moe/test.png".into(), Ok(true), reused);
        assert_eq!(f.button_label(), "Link copied");
        assert_eq!(f.heading(), "Link copied to clipboard");
        assert!(f.message().contains("copied"));
        assert!(f.hint().contains("Ready to paste"));
    }
}

#[test]
fn progress_is_visible_before_upload_and_does_not_claim_success() {
    for f in [F::Preparing, F::Uploading, F::Copying, F::Busy] {
        assert!(f.busy() && f.visible());
        assert_ne!(f.button_label(), "Link copied");
        assert!(!f.heading().contains("clipboard"));
    }
    assert_eq!(F::Failed("Offline".into()).button_label(), "Retry");
    assert!(F::Cooldown(5).message().contains("5s"));
}

#[test]
fn upload_card_aligns_with_upload_and_stays_inside_both_windows() {
    let mut m = Model::new(Default::default(), "Ctrl+Shift+S".into());
    assert_eq!(m.height(), COMPACT_HEIGHT);
    m.upload_feedback = F::Copied {
        url: "https://files.catbox.moe/test.png".into(),
        reused: false,
    };
    for (width, editor) in [(WIDTH as f32, false), (660., true), (940., true)] {
        let r = m.feedback_rect(width, editor);
        let button = m
            .bar_targets(width, editor)
            .into_iter()
            .find(|t| t.action == Action::Upload);
        // Home upload is only enabled once a screenshot exists.
        if let Some(button) = button {
            assert_eq!(r.x + r.w, button.rect.x + button.rect.w);
        }
        assert!(r.x >= 0. && r.x + r.w <= width && r.y > 62.);
        if !editor {
            assert!(r.y + r.h < m.height() as f32);
        }
        let targets = m.feedback_targets(width, editor);
        let close = targets
            .iter()
            .find(|t| t.action == Action::DismissUploadFeedback)
            .unwrap();
        assert!(r.contains(Point::new(
            close.rect.x + close.rect.w,
            close.rect.y + close.rect.h
        )));
    }
    m.menu = Menu::Options;
    assert!(m.feedback_targets(660., false).is_empty());
    m.menu = Menu::None;
    m.upload_feedback = F::None;
    assert_eq!(m.height(), COMPACT_HEIGHT);
}

#[test]
fn card_body_blocks_image_clicks_but_close_wins_hit_test() {
    let mut m = Model::new(Default::default(), String::new());
    m.has_image = true;
    m.upload_feedback = F::Failed("Offline".into());
    let r = m.feedback_rect(660., false);
    assert_eq!(
        m.hit(Point::new(r.x + 10., r.y + 60.)),
        Some(Action::UploadFeedbackBody)
    );
    let close = m.feedback_targets(660., false)[0];
    assert_eq!(
        m.hit(Point::new(close.rect.x + 10., close.rect.y + 10.)),
        Some(Action::DismissUploadFeedback)
    );
    let screen = snipping_tool_ul::capture::CapturedScreen {
        pixels: vec![255; 400 * 180 * 4],
        width: 400,
        height: 180,
        x_offset: 0,
        y_offset: 0,
        scale_factor: 1.,
    };
    let mut o = snipping_tool_ul::state::OverlayState::new(screen, &Config::default());
    o.preview_mode = true;
    o.selection = Some(snipping_tool_ul::overlay::selection::Selection {
        x: 0.,
        y: 0.,
        width: 400.,
        height: 180.,
    });
    let mut ui = snipping_tool_ul::editor_ui::EditorUi::new(&Config::default());
    ui.header.upload_feedback = m.upload_feedback;
    let r = ui.header.feedback_rect(660., true);
    assert_eq!(
        ui.hit(Point::new(r.x + 10., r.y + 60.), 660., 390., &o),
        Some(Action::UploadFeedbackBody)
    );
    let before = o.flattened_selection().unwrap();
    let mut frame = tiny_skia::Pixmap::new(660, 390).unwrap();
    ui.draw_base(&mut frame, 1.);
    ui.draw_controls(&mut frame, 1., &mut o, 1.);
    assert_eq!(
        o.flattened_selection().unwrap(),
        before,
        "Upload chrome must not affect screenshot exports"
    );
}

#[test]
fn escape_only_dismisses_feedback_when_its_card_is_actually_visible() {
    let mut m = Model::new(Default::default(), String::new());
    m.upload_feedback = F::Copied {
        url: "https://files.catbox.moe/test.png".into(),
        reused: false,
    };
    assert!(m.escape_dismisses_feedback());
    for menu in [
        Menu::Options,
        Menu::Delay,
        Menu::Modes,
        Menu::UploadConfirm,
        Menu::Hosts,
    ] {
        m.menu = menu;
        assert!(
            !m.escape_dismisses_feedback(),
            "An open menu must receive Escape first"
        );
        assert!(m.feedback_targets(660., false).is_empty());
    }
    m.menu = Menu::None;
    m.connecting = true;
    assert!(!m.escape_dismisses_feedback());
    m.connecting = false;
    m.upload_feedback = F::Uploading;
    assert!(!m.escape_dismisses_feedback());
    m.upload_feedback = F::None;
    assert!(!m.escape_dismisses_feedback());
}
