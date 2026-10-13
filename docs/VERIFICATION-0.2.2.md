# UI organization verification

This release keeps the approved dark frosted design and native event-driven renderer. Options now groups capture preferences, sharing and app navigation. Menu rows have icons. Annotation controls have visible clickable names, configured shortcut help and grouped overflow actions. Caption colors and a crop-mark icon match the command band while Windows retains its caption behavior.

Headless synthetic home/editor renders were inspected, including grouped menus, hover help, the 660 by 390 minimum editor and physical DPI scaling. These are application-client renders with an illustrative background; they do not show real DWM glass or the operating system caption.

The guarded private-desktop probe checks that Windows accepts caption, text and border color settings on its own synthetic HWND only. It never switches the input desktop and performs no capture, clipboard, hotkey, tray, notification or upload operation. A rejected probe read-back query was corrected to use the documented setters. Native visual acceptance and complete capture/paste behavior remain separate, unverified gates.

No new UI framework, network action, startup registration or permanent rendering loop is introduced. Custom raster controls are not screen-reader verified. Existing Windows caption controls remain native.

167 headless tests passed, including physical-resolution text detail, logical settings hit targets across DPI, minimum editor reachability and non-overlapping commands. Format check passed. The normal-desktop native probe refused before GUI creation; ten guarded private-desktop cycles passed with all caption theme setters accepted and the input desktop unchanged.

All four actual settings tabs were visually inspected headlessly at 100/150/200 percent. Settings now draws shared text/icons into a reused physical framebuffer; its former nearest-neighbor enlargement and per-redraw framebuffer allocation are removed. The authored logo has embedded straight RGBA pixels for title-bar/tray use and a cached native window icon, avoiding runtime SVG parsing and PNG decoding for those uses.

Initial release CPU measurements: home warm median 0.187 ms; settings at 100/150/200 percent 0.202/0.335/0.479 ms. Editor frames for 1080p and 4K captures at a 1410 by 918 viewport were 6.444 and 10.009 ms. These measure synthetic CPU rendering only, exclude native presentation/capture/clipboard, and reflect current machine load. They do not establish faster total capture latency or full-product memory use than previous builds.

Windows caption implementation follows the [DWM window attributes documentation](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute). Native drag/resize/snap controls remain owned by Windows; active glass appearance has not been visually verified.
