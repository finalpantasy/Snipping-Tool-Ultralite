# Snipping Tool UL

<!-- impeccable:product-schema 1 -->

## Platform

Windows 11 native desktop. This platform is outside Impeccable's web/iOS/Android presets; use Windows window controls and native Windows behavior.

## Users

People taking frequent screenshots while working on their PC, then pasting into chats or sharing an image link.

## Product Purpose

A lightweight replacement for Windows Snipping Tool. Ultralight in performance, not features.

## Stack

Existing native Rust, Win32 capture, winit windows and CPU rendering. No browser runtime.

## Operating Context

Launch from Start, a hotkey or tray. Select a region, copy immediately, then automatically show an annotation preview. Wheel zoom stays anchored at the pointer. Copy updates the image without closing the preview. Timers, window/full-screen capture, save and explicit optional upload remain available.

## Capabilities and Constraints

Preserve capture, clipboard, annotations, undo/redo, zoom, hotkeys, timers, OCR, pinning, opt-in save/history and manual uploads. Upload must work on click without setup and copy a direct image link suitable for Reddit/RES; anonymous Catbox is the default and Imgur remains optional. Magnifier excluded. Scrolling capture deferred. Keep heavy work out of the idle path. No public test upload, install, startup changes or publication during design review.

Tests and previews must stay headless while the user works. Native acceptance requires an isolated environment. Do not open the app or capture the active desktop for design evidence.

## Brand Commitments

Name: Snipping Tool UL. Approved direction: stronger frosted light and dark themes with restrained controls. The UL monogram and subsequent ten icon proposals were rejected; final logo remains undecided. Preserve familiar Windows controls and screenshot-tool behavior. Width, text font/size and step size are visible while editing; ordinary wheel input stays zoom. Share visual ideas before production implementation.

## Evidence on Hand

Standalone native source, 175 passing headless tests, mocked installer checks and source/asset/license notices. Synthetic editor/settings renders cover minimum/default sizes and physical DPI. A guarded private-desktop home presentation probe passes without switching input desktops. These do not prove full native capture/paste or interaction; docs/VERIFICATION-0.3.0.md records the remaining boundaries.

## Product Principles

- Screenshot and task come first; interface stays quiet.
- Immediate copying and automatic preview are the default.
- Extra capabilities remain findable without crowding the main surface.
- Runtime speed and memory matter alongside visual quality.

Product facts above come from the user's existing interview and corrections. No new audience, platform, stack or feature assumptions are needed for this design pass.
