---
name: Snipping Tool UL
description: A compact native screenshot workspace in dark graphite.
colors:
  primary: "#007c91"
  background: "#242329"
  canvas: "#19191d"
  text: "#edeaf3"
  secondary-text: "#bdb7c9"
typography:
  body:
    fontFamily: "Open Sans"
    fontSize: "13px"
    fontWeight: 400
rounded:
  control: "6px"
  menu: "10px"
spacing:
  edge: "18px"
  gap: "8px"
components:
  primary-button:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    height: "32px"
---

## Overview

The user approved the dark frosted capture/editor concepts in design/frosted. Implemented clients use quiet graphite and familiar compact controls. The screenshot is the main editor content. Ultralight means performance, not removal of annotation features.

## Colors

Teal marks primary buttons, sliders and selected controls. The scissors reference supplies the hue; opaque button fill is #007c91 with #f9fbf8 labels (4.70:1 contrast). Selection and focus use aqua #53c8d6 in dark mode and deeper teal #006a7c in light mode. Dark is the default, using teal charcoal and soft white; optional light mode uses pale teal neutrals and dark ink. Saved theme choices are preserved. Body tint uses alpha 180/255 in dark mode and 210/255 in light mode. Brightness is bounded to keep secondary labels above 4.5:1 contrast over solid white/black backdrop extremes. Nearly opaque menus and raised controls protect legibility, while the workspace and image well retain translucent material. Failure status uses a warm color. The transparent faceted bird is used for app, caption and taskbar resources. The tray uses the supplied silhouette in adaptive teal. Original tiled source artwork is preserved.

## Typography

Open Sans, bundled under its license, is the only typeface: interface labels and annotation text both use it, and no system fonts are read. Annotation text has slider, numeric size and minus/plus controls. Labels are 13 logical pixels, secondary labels 11, the empty instruction 15. Rasterization follows physical display scale; button text centers on actual glyph bounds.

## Layout

Home is 660 by 372 logical client pixels. The native caption and client command band always share the same solid theme base color, including when acrylic is available. A subtle separator leads into the frosted workspace. Windows still owns caption buttons, dragging, resizing and snap behavior. Editor defaults to 940 by 612, with 660 by 390 minimum. Source pixels remain separate from viewport chrome and exports. Copy/Save/Upload remain visible; the centered annotation strip holds common tools and overflow holds the rest. Menus consume outside clicks before the image receives them.

## Elevation & Depth

Windows DWM supplies the requested acrylic backdrop and frame. Software Direct2D presents alpha without a per-window GPU device; unsupported or disabled composition falls back to opaque softbuffer. Frames redraw only on events. Headless previews show actual client code over an illustrative gradient, not real DWM glass. Private-desktop presentation passed, but active composition appearance remains unverified.

## Shapes

Small radii and simple line icons. Restrained rounded menus and image well. No decorative cards, giant title, glow, animated entrance or fake caption buttons.

## Components

Shared New/mode/delay controls. Home has a labeled Options button; editor uses the same gear icon with hover/focus help. Options groups After capture, Sharing and App, with an icon beside each row. Delay replaces the ambiguous None label. The annotation strip keeps short visible tool names below icons; these labels are clickable. More annotations and Image actions are separate groups; Extract text and Pin on screen name their outcomes. Color choices show real swatches. Pointer and keyboard focus reveal tool help and configured shortcuts, without a timer or permanent redraw loop. Home Upload is disabled without an image. Upload needs no setup: anonymous Catbox is the default; Imgur is optional. Copy commits pending text and exports annotations. Pointer-centered wheel zoom leaves controls fixed. Overflow preserves all annotation/OCR/pin actions. System caption controls remain native.

Settings has a Windows tab using the same quiet controls. Takeover is an explicit action with a consequence confirmation, not a preference that silently removes an app. Progress disables its actions across close/reopen, and errors remain visible. Restore opens the official Store page. The optional native setup dialog has an unchecked replacement checkbox; it is setup-only and does not add background rendering.

Settings renders the shared font and icons directly at physical resolution. Its framebuffer and icon cache are reused. Tools uses actual icons and checkboxes; the entire row toggles a tool. Settings uses a gear beside its title. The supplied teal bird/scissors artwork is the app logo: all nine authored ICO sizes are embedded for Windows, and the cached window icon uses its 64px frame. The dedicated tray silhouette uses prepared 32px pixels, with white or dark ink selected from the Windows taskbar theme at startup. Window and tray startup require no SVG parsing or PNG decoding.

The bottom toolbar includes contextual width/size controls above the annotation strip. Selecting a mark edits its existing properties. Right-click deletion of a numbered step renumbers later steps, with one grouped undo. Pixelate/highlight guides exist only in the preview. Rounded pencil curves preserve dense input samples. Preview rendering caches the visible source region and draws annotations at viewport scale; export renders the source resolution separately.

Action buttons center their combined icon/text bounds. Bottom labels are 10.5 logical pixels, with Zoom naming its percentage control. Active tools have a theme-aware outline; keyboard focus uses a separate outline with space around the whole hit target. Pointer input clears and redraws keyboard focus. Empty status messages show no confirmation glyph.

## Do's and Don'ts

Preserve the approved direction and screenshot workflow. Use shared rendering and geometry for new controls. Do not add a permanent animation loop or webview. Verify visuals headlessly; never launch fixtures on the working desktop. Custom raster controls are not screen-reader verified; accessibility semantics and full keyboard annotation operation remain unaudited.


Options uses three compact groups (Capture, Sharing and App), explicit On/Off state badges, host status and navigation chevrons. Quit Snipping Tool UL is a separate final command in both home Options and the editor gear menu. Settings > General offers When closing: Keep running in tray / Quit completely; the choice applies to the home and editor X, with tray behavior retained for older configs. Dialog close/cancel behavior is unchanged. Settings grows by 40 logical pixels to keep the new row separated from Save & Close.

