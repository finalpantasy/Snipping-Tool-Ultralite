//! Offline tests of the exact production multipart requests and response contracts.
use snipping_tool_ul::{
    config::Config,
    upload::{
        image_key_for_host, no_setup_link, no_setup_request, UploadDecision, UploadHost,
        UploadSession,
    },
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nsynthetic image";

#[test]
fn every_no_setup_destination_has_the_right_endpoint_fields_and_image_bytes() {
    for host in UploadHost::ALL {
        let request = no_setup_request(PNG, host, "ULBoundary").unwrap();
        assert!(request.endpoint.starts_with("https://"));
        assert_eq!(
            request.content_type,
            "multipart/form-data; boundary=ULBoundary"
        );
        assert!(request.body.windows(PNG.len()).any(|bytes| bytes == PNG));
        let body = String::from_utf8_lossy(&request.body);
        assert!(!body.contains("userhash"));
        assert!(!body.contains("Authorization"));
        assert!(body.ends_with("\r\n--ULBoundary--\r\n"));
        if host == UploadHost::Freeimage {
            assert_eq!(request.endpoint, "https://freeimage.host/api/1/upload");
            assert!(body.contains("name=\"source\"; filename=\"capture.png\""));
            assert!(body.contains("name=\"action\"\r\n\r\nupload\r\n"));
            assert!(body.contains("name=\"key\"\r\n\r\n6d207e02198a847aa98d0a2a901485a5\r\n"));
        } else {
            assert!(body.contains("name=\"fileToUpload\"; filename=\"capture.png\""));
            assert!(body.contains("name=\"reqtype\"\r\n\r\nfileupload\r\n"));
            assert!(!body.contains("name=\"key\""));
            if let Some(expiry) = host.expiry() {
                assert_eq!(
                    request.endpoint,
                    "https://litterbox.catbox.moe/resources/internals/api.php"
                );
                assert!(body.contains(&format!("name=\"time\"\r\n\r\n{expiry}\r\n")));
            } else {
                assert_eq!(request.endpoint, "https://catbox.moe/user/api.php");
            }
        }
        assert!(no_setup_request(b"not png", host, "ULBoundary").is_err());
        assert!(no_setup_request(PNG, host, "bad\r\nboundary").is_err());
        assert!(no_setup_request(PNG, host, "synthetic").is_err());
    }
}

#[test]
fn replies_only_accept_direct_https_png_links_from_the_selected_host() {
    assert_eq!(
        no_setup_link(" https://files.catbox.moe/test.png\n", UploadHost::Catbox).unwrap(),
        "https://files.catbox.moe/test.png"
    );
    for host in [
        UploadHost::Litterbox1h,
        UploadHost::Litterbox12h,
        UploadHost::Litterbox24h,
        UploadHost::Litterbox72h,
    ] {
        assert_eq!(
            no_setup_link("https://litter.catbox.moe/test.png", host).unwrap(),
            "https://litter.catbox.moe/test.png"
        );
        assert!(no_setup_link("https://files.catbox.moe/test.png", host).is_err());
    }
    for url in [
        "https://iili.io/test.png",
        "https://freeimage.host/images/2026/10/12/test.png",
    ] {
        let reply = serde_json::json!({"status_code":200,"image":{"url":url}}).to_string();
        assert_eq!(no_setup_link(&reply, UploadHost::Freeimage).unwrap(), url);
    }
    for url in [
        "http://iili.io/test.png",
        "https://iili.io.evil.test/test.png",
        "https://iili.io@evil.test/test.png",
        "https://iili.io/../test.png",
        "https://iili.io/test.png?redirect=evil",
        "https://iili.io//evil/test.png",
        "https://iili.io/test.jpg",
        "javascript:bad.png",
    ] {
        let reply = serde_json::json!({"status_code":200,"image":{"url":url}}).to_string();
        assert!(
            no_setup_link(&reply, UploadHost::Freeimage).is_err(),
            "{url}"
        );
    }
    for reply in [
        "error",
        "{}",
        "{\"status_code\":400,\"image\":{\"url\":\"https://iili.io/test.png\"}}",
    ] {
        assert!(no_setup_link(reply, UploadHost::Freeimage).is_err());
    }
    for host in UploadHost::ALL {
        assert!(no_setup_link(&"x".repeat(65_537), host).is_err());
    }
    assert!(no_setup_link("https://litter.catbox.moe/test.png", UploadHost::Catbox).is_err());
    assert!(no_setup_link("https://evil.test/test.png", UploadHost::Litterbox1h).is_err());
}

#[test]
fn host_and_expiration_choices_are_persisted_and_never_share_a_cached_identity() {
    let mut keys = HashSet::new();
    for host in UploadHost::ALL {
        let mut config = Config::default();
        config.workflow.upload_host = host;
        let roundtrip: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(roundtrip.workflow.upload_host, host);
        assert!(keys.insert(image_key_for_host(PNG, 1, 1, host)));
    }
    let old: Config = toml::from_str("[workflow]\nconfirm_upload=true").unwrap();
    assert_eq!(old.workflow.upload_host, UploadHost::Catbox);
    assert!(old.workflow.confirm_upload);
}

#[test]
fn temporary_link_reuse_ends_at_expiration_and_host_switching_cannot_bypass_cooldown() {
    let now = Instant::now();
    for (host, hours) in [
        (UploadHost::Litterbox1h, 1),
        (UploadHost::Litterbox12h, 12),
        (UploadHost::Litterbox24h, 24),
        (UploadHost::Litterbox72h, 72),
    ] {
        let mut session = UploadSession::default();
        let key = image_key_for_host(PNG, 1, 1, host);
        assert_eq!(
            session.request_for_host_at(key, false, false, host, now),
            UploadDecision::Start
        );
        assert!(session.finish(key, Some("existing".into())));
        let other = image_key_for_host(PNG, 1, 1, UploadHost::Freeimage);
        assert_eq!(
            session.request_for_host_at(
                other,
                false,
                false,
                UploadHost::Freeimage,
                now + Duration::from_secs(1)
            ),
            UploadDecision::Cooldown(Duration::from_secs(4))
        );
        let expires = now + Duration::from_secs(hours * 3600);
        assert_eq!(
            session.request_for_host_at(key, true, false, host, expires - Duration::from_millis(1)),
            UploadDecision::Reuse("existing".into())
        );
        assert_eq!(
            session.request_for_host_at(key, true, false, host, expires),
            UploadDecision::Confirm
        );
        assert_eq!(
            session.request_for_host_at(key, true, true, host, expires),
            UploadDecision::Start
        );
    }
}

#[test]
fn hosts_are_mouse_and_keyboard_reachable_and_selection_returns_to_options() {
    use snipping_tool_ul::{
        geometry::Point,
        launcher::{Action, Menu, Model},
    };
    let mut model = Model::new(Default::default(), String::new());
    assert!(model.activate(Action::Connect));
    assert_eq!(model.menu, Menu::Hosts);
    let targets = model.menu_targets(660.);
    assert_eq!(targets.len(), UploadHost::ALL.len());
    for target in targets {
        assert!(target.rect.y + target.rect.h < 372.);
        assert_eq!(
            model.hit(Point::new(target.rect.x + 10., target.rect.y + 10.)),
            Some(target.action)
        );
        model.navigate(1, 660.);
        assert_eq!(model.enter(660.), Some(target.action));
    }
    assert!(!model.activate(Action::SetHost(UploadHost::Freeimage)));
    assert_eq!(model.workflow.upload_host, UploadHost::Freeimage);
    assert_eq!(model.menu, Menu::Options);
}
