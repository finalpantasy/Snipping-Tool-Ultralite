//! Small typed upload feedback. No timer, animation, image buffers or network work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum UploadFeedback {
    #[default]
    None,
    Preparing,
    Busy,
    Uploading,
    Copying,
    Copied {
        url: String,
        reused: bool,
    },
    Uploaded {
        url: String,
        reason: &'static str,
    },
    Failed(String),
    Cooldown(u64),
}
impl UploadFeedback {
    pub fn busy(&self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Busy | Self::Uploading | Self::Copying
        )
    }
    pub fn visible(&self) -> bool {
        !matches!(self, Self::None)
    }
    pub fn completed(url: String, copied: Result<bool, ()>, reused: bool) -> Self {
        match copied {
            Ok(true) => Self::Copied { url, reused },
            Ok(false) => Self::Uploaded {
                url,
                reason: "Newer clipboard content was kept.",
            },
            Err(()) => Self::Uploaded {
                url,
                reason: "The link could not be copied.",
            },
        }
    }
    pub fn button_label(&self) -> &'static str {
        match self {
            Self::Preparing => "Preparing…",
            Self::Uploading | Self::Busy => "Uploading…",
            Self::Copying => "Copying…",
            Self::Copied { .. } => "Link copied",
            Self::Uploaded { .. } => "Copy link",
            Self::Failed(_) => "Retry",
            _ => "Upload",
        }
    }
    pub fn heading(&self) -> &'static str {
        match self {
            Self::Preparing => "Preparing your image",
            Self::Uploading => "Uploading your image",
            Self::Busy => "An upload is already running",
            Self::Copying => "Copying the saved link",
            Self::Copied { .. } => "Link copied to clipboard",
            Self::Uploaded { .. } => "Uploaded; link not copied",
            Self::Failed(_) => "Upload failed",
            Self::Cooldown(_) => "Please wait before uploading",
            Self::None => "",
        }
    }
    pub fn detail(&self) -> &str {
        match self {
            Self::Copied { url, .. } | Self::Uploaded { url, .. } => url,
            Self::Failed(error) => error,
            Self::Preparing => "Your click was received. Preparing the image…",
            Self::Uploading => "Creating a public image link…",
            Self::Busy => "Wait for it to finish before uploading this image.",
            Self::Copying => "Reusing the existing upload. No duplicate upload.",
            Self::Cooldown(_) => "The image is still here. Try again shortly.",
            Self::None => "",
        }
    }
    pub fn hint(&self) -> &str {
        match self {
            Self::Copied { reused: true, .. } => {
                "Saved link reused. Ready to paste into your chat."
            }
            Self::Copied { .. } => "Ready to paste into your chat.",
            Self::Uploaded { reason, .. } => reason,
            Self::Failed(_) => "Image kept. Click Retry to try again.",
            _ => "",
        }
    }
    pub fn message(&self) -> String {
        match self {
            Self::Copied { url, reused } => format!(
                "{}: {url}",
                if *reused {
                    "Existing link copied"
                } else {
                    "Link copied"
                }
            ),
            Self::Uploaded { url, reason } => {
                format!("Uploaded: {url} {reason} Click Copy link to retry.")
            }
            Self::Failed(error) => format!("Upload failed: {error}. Image kept; click Retry."),
            Self::Cooldown(seconds) => format!("Please wait {seconds}s before another upload."),
            _ => self.heading().into(),
        }
    }
}
