use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "snipping-tool-ul",
    version,
    about = "Screenshot capture and annotation tool"
)]
pub struct Cli {
    /// Start quietly in the tray, without opening the home window
    #[arg(long)]
    pub background: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Capture a screenshot
    Capture {
        /// Copy full screen to clipboard (non-interactive)
        #[arg(long)]
        clipboard: bool,

        /// Save full screen to file path (non-interactive)
        #[arg(long)]
        save: Option<String>,

        /// Delay in seconds before capturing
        #[arg(long, default_value_t = 0)]
        delay: u64,

        /// Physical desktop x,y,width,height; negative origins are supported
        #[arg(long, allow_hyphen_values = true)]
        region: Option<crate::capture::ScreenRect>,
    },

    /// Install for the current user; preview unless --apply is given
    Install {
        /// Install UL in a permanent current-user folder
        #[arg(long)]
        apply: bool,
        /// Replace Microsoft's Snipping Tool in Start search for this account
        #[arg(long)]
        replace_windows_snipping_tool: bool,
        /// Show installer with optional replacement checkbox; requires --apply
        #[arg(long, requires = "apply")]
        interactive: bool,
    },

    /// Disabled in this portable build
    Uninstall,
}
