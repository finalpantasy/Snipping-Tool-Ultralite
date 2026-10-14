//! Weekly update check: one HTTPS request to GitHub's releases list at app
//! start, at most once every seven days. It never downloads or installs
//! anything; a newer release only offers a link to its page.
use std::time::Duration;

/// GitHub's release list for this app, prereleases included.
pub const RELEASES_API: &str =
    "https://api.github.com/repos/finalpantasy/Snipping-Tool-Ultralite/releases?per_page=10";
/// Release pages that may be opened. Anything else in a reply is ignored.
const RELEASE_PAGE_PREFIX: &str = "https://github.com/finalpantasy/Snipping-Tool-Ultralite/";
/// Seven days, in seconds.
pub const CHECK_INTERVAL_SECS: u64 = 7 * 24 * 60 * 60;

/// A release newer than the running app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Version without the leading "v" or tag suffix, for example "0.3.24".
    pub version: String,
    /// The release page on GitHub.
    pub url: String,
}

/// Numeric version parts. "v0.3.10-preview" gives [0, 3, 10]; a suffix
/// after "-" or "+" is ignored.
pub fn parse_version(text: &str) -> Option<Vec<u64>> {
    let text = text.trim();
    let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
    let core = text.split(['-', '+']).next()?;
    let parts = core
        .split('.')
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;
    (!parts.is_empty()).then_some(parts)
}

/// True when `candidate` is a higher version than `current`, part by part,
/// so 0.3.10 is newer than 0.3.9. Missing parts count as zero.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let (Some(a), Some(b)) = (parse_version(candidate), parse_version(current)) else {
        return false;
    };
    let len = a.len().max(b.len());
    let part = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    (0..len)
        .map(|i| part(&a, i).cmp(&part(&b, i)))
        .find(|o| o.is_ne())
        .is_some_and(|o| o.is_gt())
}

/// Whether a week has passed since `last_check` (Unix seconds). A last check
/// in the future means the clock moved back, so the check is allowed again.
pub fn check_due(last_check: u64, now: u64) -> bool {
    last_check > now || now - last_check >= CHECK_INTERVAL_SECS
}

/// The newest non-draft release in a GitHub releases reply that is newer than
/// `current`, or `None`.
pub fn newest_release(reply: &str, current: &str) -> Option<Release> {
    let releases: Vec<serde_json::Value> = serde_json::from_str(reply).ok()?;
    releases
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            let url = r["html_url"].as_str()?;
            let version = parse_version(tag)?;
            url.starts_with(RELEASE_PAGE_PREFIX)
                .then(|| (version, url.to_string()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(v, url)| Release {
            version: v.iter().map(u64::to_string).collect::<Vec<_>>().join("."),
            url,
        })
        .filter(|r| is_newer(&r.version, current))
}

/// Asks GitHub once, with a short timeout. Any failure returns `None`.
pub fn fetch_newer_release() -> Option<Release> {
    let current = env!("CARGO_PKG_VERSION");
    let reply = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()
        .get(RELEASES_API)
        .set("Accept", "application/vnd.github+json")
        .set(
            "User-Agent",
            concat!("Snipping-Tool-UL/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .ok()?
        .into_string()
        .ok()?;
    newest_release(&reply, current)
}

/// Opens a release page in the default browser. Only GitHub pages for this
/// app are accepted.
pub fn open_release_page(url: &str) -> Result<(), String> {
    if !url.starts_with(RELEASE_PAGE_PREFIX) {
        return Err("Not a Snipping Tool UL release page".into());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let explorer = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .ok_or("Windows folder unavailable")?
            .join("explorer.exe");
        std::process::Command::new(explorer)
            .arg(url)
            .creation_flags(0x08000000)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening a browser is Windows-only".into())
    }
}
