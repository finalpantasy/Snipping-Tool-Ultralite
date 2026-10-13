//! Explicit current-user integration, run only from consented Settings or install.
use crate::{
    geometry::Point,
    icons::IconCache,
    ui::{self, Canvas, Rect},
};
use std::path::Path;

pub const TAKEOVER_BUTTON: Rect = Rect {
    x: 24.,
    y: 170.,
    w: 264.,
    h: 34.,
};
pub const RESTORE_BUTTON: Rect = Rect {
    x: 24.,
    y: 310.,
    w: 264.,
    h: 34.,
};

pub fn draw(p: &mut tiny_skia::Pixmap, pending: bool, status: &str, hovered: Option<Point>) {
    let mut icons = IconCache::new();
    draw_cached(p, pending, status, hovered, &mut icons);
}
pub fn draw_cached(
    p: &mut tiny_skia::Pixmap,
    pending: bool,
    status: &str,
    hovered: Option<Point>,
    icons: &mut IconCache,
) {
    let scale = p.width() as f32 / crate::settings_ui::WIN_W as f32;
    let mut c = Canvas {
        pixmap: p,
        scale,
        icons,
    };
    c.label(24., 106., "Windows integration", 18., ui::ink());
    c.label(
        24.,
        137.,
        "Make UL the Snipping Tool result in Start search.",
        12.,
        ui::muted(),
    );
    c.button(
        TAKEOVER_BUTTON,
        if pending {
            "Applying..."
        } else {
            "Use as my Snipping Tool"
        },
        None,
        true,
        hovered.is_some_and(|p| TAKEOVER_BUTTON.contains(p)),
        !pending,
    );
    c.label(
        24.,
        217.,
        "Installs UL for this account, then removes",
        12.,
        ui::muted(),
    );
    c.label(
        24.,
        236.,
        "Microsoft's Snipping Tool for this account.",
        12.,
        ui::muted(),
    );
    c.label(
        24.,
        260.,
        "Win+Shift+S is not remapped. Capture hotkey is",
        11.,
        ui::muted(),
    );
    c.label(
        24.,
        278.,
        "configured in General. Startup stays optional.",
        11.,
        ui::muted(),
    );
    c.button(
        RESTORE_BUTTON,
        "Restore Microsoft from Store",
        None,
        false,
        hovered.is_some_and(|p| RESTORE_BUTTON.contains(p)),
        !pending,
    );
    for (i, line) in status
        .chars()
        .collect::<Vec<_>>()
        .chunks(48)
        .take(3)
        .enumerate()
    {
        c.label(
            24.,
            362. + i as f32 * 18.,
            &line.iter().collect::<String>(),
            11.,
            ui::muted(),
        );
    }
}

pub fn install(
    exe: &Path,
    replace: bool,
    apply: bool,
    interactive: bool,
) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::{io::Write, os::windows::process::CommandExt, process::Command};
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let workspace = std::env::temp_dir().join(format!(
            "SnippingToolUL.Install.{}.{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&workspace).map_err(|e| e.to_string())?;
        struct OwnedWorkspace(std::path::PathBuf);
        impl Drop for OwnedWorkspace {
            fn drop(&mut self) {
                for name in [
                    "installer.ps1",
                    "LICENSE",
                    "THIRD_PARTY_NOTICES.md",
                    "assets/OpenSans-OFL.txt",
                    "assets/Lucide-LICENSE.txt",
                    "docs/DEPENDENCY_NOTICES.md",
                ] {
                    let _ = std::fs::remove_file(self.0.join(name));
                }
                for name in ["assets", "docs"] {
                    let _ = std::fs::remove_dir(self.0.join(name));
                }
                let _ = std::fs::remove_dir(&self.0);
            }
        }
        let owned = OwnedWorkspace(workspace);
        let notices = &owned.0;
        let temp = owned.0.join("installer.ps1");
        for (name, data) in [
            ("LICENSE", include_bytes!("../LICENSE") as &[u8]),
            (
                "THIRD_PARTY_NOTICES.md",
                include_bytes!("../THIRD_PARTY_NOTICES.md") as &[u8],
            ),
            (
                "assets/OpenSans-OFL.txt",
                include_bytes!("../assets/OpenSans-OFL.txt") as &[u8],
            ),
            (
                "assets/Lucide-LICENSE.txt",
                include_bytes!("../assets/Lucide-LICENSE.txt") as &[u8],
            ),
            (
                "docs/DEPENDENCY_NOTICES.md",
                include_bytes!("../docs/DEPENDENCY_NOTICES.md") as &[u8],
            ),
        ] {
            let path = notices.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&path, data).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(include_bytes!("../scripts/Install-CurrentUser.ps1"))
            .map_err(|e| e.to_string())?;
        drop(file);
        let powershell = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .ok_or("Windows folder unavailable")?
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command = Command::new(powershell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&temp)
            .arg("-Executable")
            .arg(exe)
            .arg("-Version")
            .arg(env!("CARGO_PKG_VERSION"))
            .arg("-NoticeFolder")
            .arg(&notices)
            .creation_flags(0x08000000);
        if replace {
            command.arg("-ReplaceWindowsSnippingTool");
        }
        if apply {
            command.arg("-Apply");
        }
        if interactive {
            command.arg("-Interactive");
        }
        let output = command.output();
        let output = output.map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(800)
                .collect());
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (exe, replace, apply, interactive);
        Err("Windows integration is Windows-only".into())
    }
}

pub fn restore_store() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::{os::windows::process::CommandExt, process::Command};
        let explorer = std::env::var_os("WINDIR")
            .map(std::path::PathBuf::from)
            .ok_or("Windows folder unavailable")?
            .join("explorer.exe");
        Command::new(explorer)
            .arg("ms-windows-store://pdp/?ProductId=9MZ95KL8MR0L")
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Windows integration is Windows-only".into())
    }
}
