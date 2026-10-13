//! Explicit image sharing. Anonymous Catbox needs no account; Imgur stays optional.
use std::io::Read;
use std::time::{Duration, Instant};

pub const UPLOAD_COOLDOWN: Duration = Duration::from_secs(5);

/// Destinations that never require a user account or user-supplied credentials.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UploadHost {
    #[default]
    Catbox,
    Freeimage,
    Litterbox1h,
    Litterbox12h,
    Litterbox24h,
    Litterbox72h,
}
impl UploadHost {
    pub const ALL: [Self; 6] = [
        Self::Catbox,
        Self::Freeimage,
        Self::Litterbox1h,
        Self::Litterbox12h,
        Self::Litterbox24h,
        Self::Litterbox72h,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Catbox => "Catbox",
            Self::Freeimage => "Freeimage.host",
            _ => "Litterbox",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Catbox => "Catbox",
            Self::Freeimage => "Freeimage.host",
            Self::Litterbox1h => "Litterbox · 1 hour",
            Self::Litterbox12h => "Litterbox · 12 hours",
            Self::Litterbox24h => "Litterbox · 1 day",
            Self::Litterbox72h => "Litterbox · 3 days",
        }
    }
    pub fn expiry(self) -> Option<&'static str> {
        match self {
            Self::Litterbox1h => Some("1h"),
            Self::Litterbox12h => Some("12h"),
            Self::Litterbox24h => Some("24h"),
            Self::Litterbox72h => Some("72h"),
            _ => None,
        }
    }
    pub fn disclosure(self) -> &'static str {
        match self {
            Self::Catbox => "Anyone with the link can view it.",
            Self::Freeimage => "Public upload; may appear on the host's website.",
            Self::Litterbox1h => "Public link expires after 1 hour.",
            Self::Litterbox12h => "Public link expires after 12 hours.",
            Self::Litterbox24h => "Public link expires after 1 day.",
            Self::Litterbox72h => "Public link expires after 3 days.",
        }
    }
}

pub fn image_key_for_host(pixels: &[u8], width: u32, height: u32, host: UploadHost) -> [u8; 32] {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(&width.to_le_bytes());
    hash.update(&height.to_le_bytes());
    hash.update(host.label().as_bytes());
    hash.update(pixels);
    hash.finish().as_ref().try_into().unwrap()
}

pub struct UploadRequest {
    pub endpoint: &'static str,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Build the exact production request without touching the network.
pub fn no_setup_request(
    png: &[u8],
    host: UploadHost,
    boundary: &str,
) -> Result<UploadRequest, String> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Expected a PNG image".into());
    }
    let (endpoint, fields, file_field) = match host {
        UploadHost::Catbox => (
            "https://catbox.moe/user/api.php",
            vec![("reqtype", "fileupload")],
            "fileToUpload",
        ),
        // This is the public shared key explicitly supplied by the host's API/ShareX guide.
        UploadHost::Freeimage => (
            "https://freeimage.host/api/1/upload",
            vec![
                ("key", "6d207e02198a847aa98d0a2a901485a5"),
                ("action", "upload"),
                ("format", "json"),
            ],
            "source",
        ),
        _ => (
            "https://litterbox.catbox.moe/resources/internals/api.php",
            vec![("reqtype", "fileupload"), ("time", host.expiry().unwrap())],
            "fileToUpload",
        ),
    };
    let limit = if host == UploadHost::Freeimage {
        64 * 1024 * 1024
    } else if host == UploadHost::Catbox {
        200 * 1024 * 1024
    } else {
        1024 * 1024 * 1024
    };
    if png.len() > limit {
        return Err(format!("Image exceeds {}'s file limit.", host.name()));
    }
    Ok(UploadRequest {
        endpoint,
        content_type: format!("multipart/form-data; boundary={boundary}"),
        body: multipart_body(png, boundary, &fields, file_field)?,
    })
}

pub fn no_setup_link(reply: &str, host: UploadHost) -> Result<String, String> {
    if reply.len() > 65_536 {
        return Err("Image host response was too large.".into());
    }
    if host == UploadHost::Freeimage {
        let data: serde_json::Value =
            serde_json::from_str(reply).map_err(|_| "Invalid Freeimage.host response.")?;
        if data["status_code"].as_u64() != Some(200) {
            return Err("Freeimage.host did not accept the image.".into());
        }
        let url = data["image"]["url"].as_str().unwrap_or("");
        let path = url
            .strip_prefix("https://iili.io/")
            .or_else(|| url.strip_prefix("https://freeimage.host/images/"));
        if path.is_some_and(valid_png_path) {
            return Ok(url.to_owned());
        }
        return Err("Freeimage.host did not return a valid PNG link.".into());
    }
    let url = reply.trim();
    let domain = if host == UploadHost::Catbox {
        "https://files.catbox.moe/"
    } else {
        "https://litter.catbox.moe/"
    };
    if url.strip_prefix(domain).is_some_and(valid_png_path) {
        Ok(url.to_owned())
    } else {
        Err(format!(
            "{} did not return a valid image link.",
            host.name()
        ))
    }
}
fn valid_png_path(path: &str) -> bool {
    path.len() >= 5
        && path.len() <= 256
        && path.ends_with(".png")
        && !path.contains("..")
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_/".contains(&b))
        && !path.starts_with('/')
        && !path.contains("//")
}

pub fn upload_no_setup(png: &[u8], host: UploadHost) -> Result<String, String> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let request = no_setup_request(png, host, &format!("SnippingToolUL{nonce:x}"))?;
    let response = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(120))
        .timeout(Duration::from_secs(180))
        .build()
        .post(request.endpoint)
        .set("Content-Type", &request.content_type)
        .send_bytes(&request.body)
        .map_err(|e| format!("{} request failed: {e}", host.name()))?;
    let reply = read_success_response(response, host.name(), 65_536)?;
    no_setup_link(&reply, host)
}

fn read_success_response(
    response: ureq::Response,
    service: &str,
    max_bytes: usize,
) -> Result<String, String> {
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(format!("{service} returned HTTP status {status}."));
    }

    let mut body = String::new();
    response
        .into_reader()
        .take(max_bytes.saturating_add(1) as u64)
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    if body.len() > max_bytes {
        return Err(format!("{service} response was too large."));
    }
    Ok(body)
}

/// Fingerprint the actual flattened image and destination, never its filename.
pub fn image_key(pixels: &[u8], width: u32, height: u32, client_id: &str) -> [u8; 32] {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(&width.to_le_bytes());
    hash.update(&height.to_le_bytes());
    hash.update(host_name(client_id).as_bytes());
    if let Ok(id) = imgur_client_id(client_id) {
        hash.update(&(id.len() as u64).to_le_bytes());
        hash.update(id.as_bytes());
    }
    hash.update(pixels);
    hash.finish().as_ref().try_into().unwrap()
}

#[derive(Debug, PartialEq, Eq)]
pub enum UploadDecision {
    Busy,
    Cooldown(Duration),
    Confirm,
    Start,
    Reuse(String),
}

/// Small in-memory link cache. Keeps no screenshot buffers or credentials.
#[derive(Default)]
pub struct UploadSession {
    awaiting: Option<[u8; 32]>,
    pending: Option<[u8; 32]>,
    last_started: Option<Instant>,
    pending_lifetime: Option<Duration>,
    links: std::collections::VecDeque<([u8; 32], String, Option<Instant>)>,
}
impl UploadSession {
    pub fn request_for_host(
        &mut self,
        key: [u8; 32],
        confirm: bool,
        explicit_accept: bool,
        host: UploadHost,
    ) -> UploadDecision {
        self.request_for_host_at(key, confirm, explicit_accept, host, Instant::now())
    }
    pub fn request_for_host_at(
        &mut self,
        key: [u8; 32],
        confirm: bool,
        explicit_accept: bool,
        host: UploadHost,
        now: Instant,
    ) -> UploadDecision {
        let decision = self.request_at(key, confirm, explicit_accept, now);
        if decision == UploadDecision::Start {
            self.pending_lifetime = match host.expiry() {
                Some("1h") => Some(Duration::from_secs(3600)),
                Some("12h") => Some(Duration::from_secs(12 * 3600)),
                Some("24h") => Some(Duration::from_secs(24 * 3600)),
                Some("72h") => Some(Duration::from_secs(72 * 3600)),
                _ => None,
            };
        }
        decision
    }
    pub fn request(
        &mut self,
        key: [u8; 32],
        confirm: bool,
        explicit_accept: bool,
    ) -> UploadDecision {
        self.request_at(key, confirm, explicit_accept, Instant::now())
    }

    /// Explicit monotonic time keeps cooldown tests deterministic, without sleeping.
    pub fn request_at(
        &mut self,
        key: [u8; 32],
        confirm: bool,
        explicit_accept: bool,
        now: Instant,
    ) -> UploadDecision {
        if self.pending.is_some() {
            return UploadDecision::Busy;
        }
        self.links
            .retain(|(_, _, expiry)| expiry.is_none_or(|expires| now < expires));
        if let Some((_, url, _)) = self.links.iter().find(|(k, _, _)| *k == key) {
            self.awaiting = None;
            return UploadDecision::Reuse(url.clone());
        }
        if confirm && !(explicit_accept && self.awaiting == Some(key)) {
            self.awaiting = Some(key);
            return UploadDecision::Confirm;
        }
        if let Some(last) = self.last_started {
            let elapsed = now.saturating_duration_since(last);
            if elapsed < UPLOAD_COOLDOWN {
                return UploadDecision::Cooldown(UPLOAD_COOLDOWN - elapsed);
            }
        }
        self.awaiting = None;
        self.pending = Some(key);
        self.last_started = Some(now);
        self.pending_lifetime = None;
        UploadDecision::Start
    }
    pub fn cancel(&mut self) {
        self.awaiting = None;
    }
    pub fn finish(&mut self, key: [u8; 32], url: Option<String>) -> bool {
        if self.pending != Some(key) {
            return false;
        }
        self.pending = None;
        let expires = self
            .pending_lifetime
            .take()
            .and_then(|duration| self.last_started?.checked_add(duration));
        if let Some(url) = url {
            self.links.push_back((key, url, expires));
            if self.links.len() > 8 {
                self.links.pop_front();
            }
        }
        true
    }
}

pub fn upload_image(png_data: &[u8], config_client_id: &str) -> Result<String, String> {
    if imgur_client_id(config_client_id).is_ok() {
        upload_to_imgur(png_data, config_client_id)
    } else {
        upload_to_catbox(png_data)
    }
}

pub fn host_name(config_client_id: &str) -> &'static str {
    if imgur_client_id(config_client_id).is_ok() {
        "Imgur"
    } else {
        "Catbox"
    }
}

pub fn upload_to_catbox(png_data: &[u8]) -> Result<String, String> {
    if !png_data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Expected a PNG image".into());
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let boundary = format!("SnippingToolUL{nonce:x}");
    let body = catbox_body(png_data, &boundary)?;
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(120))
        .timeout(Duration::from_secs(180))
        .build();
    let response = agent
        .post("https://catbox.moe/user/api.php")
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&body)
        .map_err(|e| format!("Catbox request failed: {e}"))?;
    let reply = read_success_response(response, "Catbox", 4096)?;
    catbox_link(&reply)
}

fn catbox_body(png: &[u8], boundary: &str) -> Result<Vec<u8>, String> {
    multipart_body(png, boundary, &[("reqtype", "fileupload")], "fileToUpload")
}
fn multipart_body(
    png: &[u8],
    boundary: &str,
    fields: &[(&str, &str)],
    file_field: &str,
) -> Result<Vec<u8>, String> {
    if boundary.is_empty()
        || !boundary.bytes().all(|b| b.is_ascii_alphanumeric())
        || png
            .windows(boundary.len())
            .any(|w| w == boundary.as_bytes())
    {
        return Err("Invalid upload boundary".into());
    }
    let mut header = String::new();
    for (name, value) in fields {
        header.push_str(&format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
        ));
    }
    header.push_str(&format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{file_field}\"; filename=\"capture.png\"\r\nContent-Type: image/png\r\n\r\n"));
    let mut out = Vec::with_capacity(header.len() + png.len() + boundary.len() + 8);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(png);
    out.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Ok(out)
}

fn catbox_link(reply: &str) -> Result<String, String> {
    let url = reply.trim();
    let path = url.strip_prefix("https://files.catbox.moe/").unwrap_or("");
    if reply.len() > 4096
        || path.len() < 5
        || path.len() > 128
        || !path.ends_with(".png")
        || !path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
        || path.contains("..")
    {
        return Err("Catbox did not return a valid image link. Try again later.".into());
    }
    Ok(url.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anonymous_form_contains_image_and_no_credentials() {
        let png = b"\x89PNG\r\n\x1a\nsynthetic bytes";
        let body = catbox_body(png, "ULTestBoundary").unwrap();
        assert!(body.windows(png.len()).any(|w| w == png));
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"reqtype\"\r\n\r\nfileupload"));
        assert!(text.contains("name=\"fileToUpload\"; filename=\"capture.png\""));
        assert!(!text.contains("userhash"));
        assert!(text.ends_with("\r\n--ULTestBoundary--\r\n"));
        assert!(catbox_body(png, "bad\r\nboundary").is_err());
        assert!(catbox_body(png, "synthetic").is_err());
    }
    #[test]
    fn server_errors_and_untrusted_links_never_become_clipboard_links() {
        assert_eq!(
            catbox_link(" https://files.catbox.moe/abc123.png\n").unwrap(),
            "https://files.catbox.moe/abc123.png"
        );
        for response in [
            "error",
            "https://evil.test/a.png",
            "https://files.catbox.moe.evil/a.png",
            "https://files.catbox.moe/../a.png",
            "https://files.catbox.moe/a.png\nhttps://evil.test",
            "http://files.catbox.moe/a.png",
        ] {
            assert!(catbox_link(response).is_err());
        }
        assert!(catbox_link(&"x".repeat(4097)).is_err());
    }
}

fn imgur_client_id(config_id: &str) -> Result<String, String> {
    // Environment variable overrides config
    if let Ok(env_id) = std::env::var("SNIPPING_TOOL_UL_IMGUR_CLIENT_ID") {
        if !env_id.is_empty() {
            return Ok(env_id);
        }
    }
    // Fall back to config setting
    if !config_id.is_empty() {
        return Ok(config_id.to_string());
    }
    Err(
        "No Imgur client ID configured. Set one in Settings > General, \
         or via the SNIPPING_TOOL_UL_IMGUR_CLIENT_ID environment variable."
            .to_string(),
    )
}

pub fn upload_to_imgur(png_data: &[u8], config_client_id: &str) -> Result<String, String> {
    let base64_image = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(png_data)
    };

    let client_id = imgur_client_id(config_client_id)?;
    let response = ureq::AgentBuilder::new()
        .redirects(0)
        .build()
        .post("https://api.imgur.com/3/image")
        .set("Authorization", &format!("Client-ID {}", client_id))
        .timeout(std::time::Duration::from_secs(30))
        .send_form(&[("image", &base64_image), ("type", "base64")])
        .map_err(|e| format!("Upload failed: {}", e))?;

    let body = read_success_response(response, "Imgur", 65_536)?;

    parse_imgur_response(&body)
}

fn parse_imgur_response(json: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| "Failed to parse Imgur response".to_string())?;
    let status = value["status"].as_u64();
    if value["success"].as_bool() != Some(true)
        || !status.is_some_and(|status| (200..300).contains(&status))
    {
        return Err("Imgur did not accept the image.".into());
    }

    let link = value["data"]["link"].as_str().unwrap_or("");
    let path = link.strip_prefix("https://i.imgur.com/").unwrap_or("");
    if valid_imgur_image_path(path) {
        Ok(link.to_owned())
    } else {
        Err("Imgur did not return a valid image link.".into())
    }
}

fn valid_imgur_image_path(path: &str) -> bool {
    let Some((name, extension)) = path.rsplit_once('.') else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 128
        && matches!(extension, "png" | "jpg" | "jpeg" | "gif" | "webp")
        && path.len() <= 256
        && !path.contains("..")
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

#[cfg(test)]
mod response_guard_tests {
    use super::*;

    fn response(status: u16, body: &str) -> ureq::Response {
        ureq::Response::new(status, "synthetic", body).unwrap()
    }

    #[test]
    fn upload_responses_require_2xx_and_enforce_a_read_limit() {
        for status in [200, 299] {
            assert_eq!(
                read_success_response(response(status, "ok"), "Synthetic", 2).unwrap(),
                "ok"
            );
        }
        for status in [199, 300, 302, 500] {
            assert!(
                read_success_response(response(status, "redirect or error"), "Synthetic", 64)
                    .unwrap_err()
                    .contains(&status.to_string())
            );
        }
        assert!(read_success_response(response(200, "oversized"), "Synthetic", 4).is_err());
    }

    #[test]
    fn imgur_response_requires_success_envelope_and_direct_https_image_link() {
        assert_eq!(
            parse_imgur_response(
                r#"{"success":true,"status":200,"data":{"link":"https://i.imgur.com/abc123.png"}}"#
            )
            .unwrap(),
            "https://i.imgur.com/abc123.png"
        );
        assert!(parse_imgur_response(
            r#"{"success":false,"status":200,"data":{"link":"https://i.imgur.com/abc123.png"}}"#
        )
        .is_err());
        assert!(parse_imgur_response(
            r#"{"success":true,"status":400,"data":{"link":"https://i.imgur.com/abc123.png"}}"#
        )
        .is_err());

        for link in [
            "http://i.imgur.com/abc123.png",
            "https://i.imgur.com.evil.test/abc123.png",
            "https://i.imgur.com@evil.test/abc123.png",
            "https://i.imgur.com/../abc123.png",
            "https://i.imgur.com/abc123.png?redirect=evil",
            "https://i.imgur.com/abc123.txt",
        ] {
            let body = serde_json::json!({
                "success": true,
                "status": 200,
                "data": {"link": link}
            })
            .to_string();
            assert!(parse_imgur_response(&body).is_err(), "{link}");
        }
    }
}
