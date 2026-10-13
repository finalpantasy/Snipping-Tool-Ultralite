# Isolated Windows acceptance checklist

Windows Sandbox is not currently enabled on the test host. It requires administrator setup and may require a restart. No enablement, elevation prompt or guest launch was attempted while the working desktop was in use.

`Prepare-SandboxTest.ps1 -Package <verified-portable-folder> -Output <new-folder>` prepares a `.wsb` file without launching it. The guest receives only the packaged application/notices and a dedicated results folder. The input mapping is read-only; the results mapping is writable. Networking, host clipboard sharing, microphone, camera and printer sharing are disabled. Nothing maps the source checkout or user profile. Configuration fields follow [Microsoft's Sandbox configuration documentation](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file).

When Sandbox is available and it is convenient to open its window, launch the generated `.wsb`. Its guest-only preflight checks the executable hash, version/help and invalid-region guard, then writes a report to Results. It does **not** claim native workflow acceptance or launch the app automatically. Open **Open Snipping Tool UL.cmd** on the guest desktop to begin. Test preferences remain inside the guest. Exiting Sandbox discards guest state; only files explicitly saved in Results survive.

Use authored, non-sensitive text and shapes as capture targets. Record the version, result and evidence filename for every row; leave untested rows pending.

| Workflow | Required observation |
| --- | --- |
| Tray About | A themed version window opens even with notifications off; repeated About focuses one window; Close, X and Escape dismiss only About |
| Capture/New | Region, window and full-screen capture work; New works on one click, cancellation restores the appropriate window |
| Clipboard | Immediate image paste works in a guest app/browser; edited Copy includes marks; clipboard contention reports an error without losing the image |
| Timers/hotkeys | 3/5/10-second countdowns, cancellation and shortcut conflicts behave correctly; failed replacement preserves a working shortcut |
| Editing | Pencil circles, arrows at several widths, text descenders, pixelation guides, selection/move/resize, undo/redo and numbered-step deletion work |
| Pan/zoom | Middle drag pans; wheel zoom stays under pointer after annotations; Space fits only when Select is active |
| History/save | History off/on status is accurate; saved images reopen, thumbnails stay sharp, autosave location is honored, Save is responsive, denied saves explain failure |
| Lifecycle | Both X settings work, full Quit removes the process/tray, no image remains after closing the editor, repeated captures do not grow memory without bound |
| Theme/DPI | Dark/light captions, icons, labels and controls match; 100/125/150/200% text remains crisp |
| Working media | Save a genuine guest screenshot/GIF of capturing/annotating to Results for the README; label it as a Sandbox session |

Sandbox does not reproduce a physical mixed-DPI multi-monitor layout or negative monitor origins. Those checks remain separate. It also cannot certify hardware-specific latency or Microsoft Snipping Tool comparisons. Record warmed medians and maximums with machine/session details before making performance claims.

Live provider/Reddit checks remain pending in this offline kit. They need a separately approved network-enabled isolated session and a clearly synthetic public upload fixture. Never enable host clipboard sharing to test paste; use applications inside the guest.
