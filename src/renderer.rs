use crate::geometry::Color;
use crate::icons::blend_pixmap;
use crate::overlay::selection::Selection;
use crate::overlay::toolbar::{ButtonAction, BUTTONS};
use crate::state::OverlayState;
use crate::tools::{
    annotation_resize_bounds, measure_text_width, render_annotation, Annotation, AnnotationTool,
    ToolKind,
};
use tiny_skia::{Paint, PathBuilder, Stroke, Transform};

fn draw_background(screen: &crate::capture::CapturedScreen, pixmap: &mut tiny_skia::Pixmap) {
    // All platform screenshot backends return opaque RGBA. Preserve row strides
    // when the preview is larger or smaller than the selected image.
    let dst_width = pixmap.width() as usize;
    let rows = (screen.height as usize).min(pixmap.height() as usize);
    let cols = (screen.width as usize).min(dst_width);
    pixmap.fill(tiny_skia::Color::from_rgba8(24, 24, 28, 255));
    for y in 0..rows {
        let start = y * screen.width as usize * 4;
        let Some(src) = screen.pixels.get(start..start + cols * 4) else {
            break;
        };
        let dst = &mut pixmap.data_mut()[y * dst_width * 4..(y * dst_width + cols) * 4];
        for (src, dst) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
            for channel in 0..3 {
                // Match tiny-skia's low-precision div255 multiplication.
                let product = src[channel] as u16 * 153;
                dst[channel] = ((product + 255) >> 8) as u8;
            }
            dst[3] = 255;
        }
    }
}

#[cfg(test)]
mod background_tests {
    use super::*;
    #[test]
    fn dimming_matches_previous_skia_composition_for_every_byte_value() {
        let pixels: Vec<_> = (0..=255u8)
            .flat_map(|v| [v, 255 - v, v.wrapping_mul(17), 255])
            .collect();
        let screen = crate::capture::CapturedScreen {
            pixels: pixels.clone(),
            width: 256,
            height: 1,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.0,
        };
        let mut reference =
            tiny_skia::Pixmap::from_vec(pixels, tiny_skia::IntSize::from_wh(256, 1).unwrap())
                .unwrap();
        let mut paint = Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
        paint.anti_alias = false;
        reference.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 256.0, 1.0).unwrap(),
            &paint,
            Transform::identity(),
            None,
        );
        let mut actual = tiny_skia::Pixmap::new(256, 1).unwrap();
        draw_background(&screen, &mut actual);
        for (i, (a, b)) in actual.data().iter().zip(reference.data()).enumerate() {
            assert_eq!(a, b, "component {i}, source {}", screen.pixels[i]);
        }
    }
    #[test]
    fn larger_preview_preserves_source_row_stride() {
        let screen = crate::capture::CapturedScreen {
            pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
            width: 1,
            height: 2,
            x_offset: 0,
            y_offset: 0,
            scale_factor: 1.0,
        };
        let mut actual = tiny_skia::Pixmap::new(3, 3).unwrap();
        draw_background(&screen, &mut actual);
        assert_eq!(&actual.data()[0..4], &[153, 0, 0, 255]);
        assert_eq!(&actual.data()[12..16], &[0, 153, 0, 255]);
        assert_eq!(&actual.data()[4..8], &[24, 24, 28, 255]);
    }
}

/// Render the full overlay frame: screenshot background, dim, selection highlight,
/// annotations, and toolbar.
pub fn render_overlay(state: &mut OverlayState, pixmap: &mut tiny_skia::Pixmap) {
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;

    // Keep one source image. Compose dimming into the reusable render target
    // rather than retaining two additional full-desktop images.
    if state.preview_mode && pixmap.data().len() == state.screenshot.pixels.len() {
        pixmap.data_mut().copy_from_slice(&state.screenshot.pixels);
    } else {
        draw_background(&state.screenshot, pixmap);
    }

    // 2b. Crosshair guide lines through cursor for precise alignment
    let show_crosshair = state.selection.is_none() || state.is_selecting;
    if show_crosshair {
        let mx = state.last_mouse_pos.x;
        let my = state.last_mouse_pos.y;
        let pw = pixmap.width() as f32;
        let ph = pixmap.height() as f32;

        let mut paint = Paint::default();
        paint.set_color(crate::theme::skia(crate::theme::accent(), 0.5));
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };

        // Vertical line
        let mut pb = PathBuilder::new();
        pb.move_to(mx, 0.0);
        pb.line_to(mx, ph);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Horizontal line
        let mut pb = PathBuilder::new();
        pb.move_to(0.0, my);
        pb.line_to(pw, my);
        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 3. If selection exists, restore brightness by copying from cached screenshot pixmap
    if let Some(sel) = &state.selection {
        let src_w = state.screenshot.width as usize;
        let sx = (sel.x as usize).min(width);
        let sy = (sel.y as usize).min(height);
        let sx2 = ((sel.x + sel.width) as usize).min(width).min(src_w);
        let sy2 = ((sel.y + sel.height) as usize).min(height);
        let row_bytes = (sx2.saturating_sub(sx)) * 4; // 4 bytes per pixel

        if row_bytes > 0 && !state.preview_mode {
            let screenshot_data = &state.screenshot.pixels;
            let pixmap_data = pixmap.data_mut();
            for y in sy..sy2 {
                let src_offset = (y * src_w + sx) * 4;
                let dst_offset = (y * width + sx) * 4;
                if src_offset + row_bytes <= screenshot_data.len()
                    && dst_offset + row_bytes <= pixmap_data.len()
                {
                    pixmap_data[dst_offset..dst_offset + row_bytes]
                        .copy_from_slice(&screenshot_data[src_offset..src_offset + row_bytes]);
                }
            }
        }

        // 4. Selection border — white 1px stroke
        if let Some(rect) = tiny_skia::Rect::from_xywh(sel.x, sel.y, sel.width, sel.height)
            .filter(|_| !state.preview_mode)
        {
            let path = PathBuilder::from_rect(rect);
            let mut paint = Paint::default();
            paint.set_color(crate::theme::skia(crate::theme::accent(), 0.9));
            paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.5,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // 4b. Selection size label
        if !state.preview_mode {
            render_size_label(pixmap, sel, width as f32, height as f32);
        }
    }

    let ss_pixels = Some(state.screenshot.pixels.as_slice());
    let ss_width = Some(state.screenshot.width);

    // 5. In-progress annotation preview
    let in_progress = match state.active_tool {
        ToolKind::Select => None,
        ToolKind::Arrow => state.arrow_tool.in_progress_annotation(),
        ToolKind::Rectangle => state.rectangle_tool.in_progress_annotation(),
        ToolKind::Circle => state.circle_tool.in_progress_annotation(),
        ToolKind::Line => state.line_tool.in_progress_annotation(),
        ToolKind::Pencil => state.pencil_tool.in_progress_annotation(),
        ToolKind::Highlight => state.highlight_tool.in_progress_annotation(),
        ToolKind::Text => state.text_tool.in_progress_annotation(),
        ToolKind::Pixelate => state.pixelate_tool.in_progress_annotation(),
        ToolKind::StepMarker => state.step_marker_tool.in_progress_annotation(),
        ToolKind::Eyedropper => None,
        ToolKind::RoundedRect => state.rounded_rect_tool.in_progress_annotation(),
        ToolKind::Spotlight => state.spotlight_tool.in_progress_annotation(),
        ToolKind::Measurement => state.measurement_tool.in_progress_annotation(),
    };

    // 6. Spotlight effect — dim everything outside spotlight rectangles. This
    // runs before the annotations are drawn so each one is composited exactly
    // once; drawing them under the dim and again on top doubled translucent
    // highlights and thickened anti-aliased edges.
    {
        let mut spotlights: Vec<(crate::geometry::Point, crate::geometry::Size)> = state
            .annotations
            .iter()
            .filter_map(|a| match a {
                Annotation::Spotlight { top_left, size } => Some((*top_left, *size)),
                _ => None,
            })
            .collect();
        // Include in-progress spotlight
        if let Some(Annotation::Spotlight { top_left, size }) = in_progress.as_ref() {
            spotlights.push((*top_left, *size));
        }
        if !spotlights.is_empty() {
            // Same routine the export path uses, so Copy/Save match the screen.
            crate::export::apply_spotlights(
                pixmap,
                &state.screenshot.pixels,
                state.screenshot.width,
                &spotlights,
            );
        }
    }

    // 6a. Finalized annotations, then the in-progress one, drawn once each
    for annotation in &state.annotations {
        render_annotation(annotation, pixmap, ss_pixels, ss_width);
    }
    if let Some(ref ann) = in_progress {
        render_annotation(ann, pixmap, ss_pixels, ss_width);
    }

    // 6b. Selection highlight around selected annotation
    if let Some(idx) = state.selected_index {
        if let Some(ann) = state.annotations.get(idx) {
            if let Some((bx, by, bw, bh)) = annotation_resize_bounds(ann) {
                render_selection_highlight(pixmap, bx - 4.0, by - 4.0, bw + 8.0, bh + 8.0);
            }
        }
    }

    // 7. Text input preview (in-progress text being typed)
    if state.text_input_active && !state.text_input_buffer.is_empty() {
        let preview = Annotation::Text {
            position: state.text_input_position,
            text: state.text_input_buffer.clone(),
            color: state.current_color,
            font_size: state.text_input_font_size,
        };
        render_annotation(&preview, pixmap, ss_pixels, ss_width);
    }
    // Text cursor (vertical white line after text)
    if state.text_input_active {
        let cursor_x = state.text_input_position.x
            + measure_text_width(&state.text_input_buffer, state.text_input_font_size);
        let cursor_y = state.text_input_position.y;
        let cursor_h = state.text_input_font_size;
        let mut pb = PathBuilder::new();
        pb.move_to(cursor_x, cursor_y);
        pb.line_to(cursor_x, cursor_y + cursor_h);
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(tiny_skia::Color::WHITE);
            paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.5,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    // 8. Toolbar (only if there is a selection)
    if let Some(sel) = state.selection.filter(|_| !state.preview_mode) {
        render_toolbar(state, &sel, pixmap);
    }

    // 9. Eyedropper preview (color swatch + hex near cursor)
    if state.active_tool == ToolKind::Eyedropper {
        if let Some(color) = state.eyedropper_preview {
            let mx = state.last_mouse_pos.x;
            let my = state.last_mouse_pos.y;

            // Small filled square showing the color
            let swatch_size = 24.0_f32;
            let swatch_x = mx + 16.0;
            let swatch_y = my + 16.0;

            if let Some(rect) =
                tiny_skia::Rect::from_xywh(swatch_x, swatch_y, swatch_size, swatch_size)
            {
                let mut paint = Paint::default();
                if let Some(c) = tiny_skia::Color::from_rgba(color.r, color.g, color.b, 1.0) {
                    paint.set_color(c);
                }
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);

                // White border around swatch
                let path = PathBuilder::from_rect(rect);
                let mut border = Paint::default();
                border.set_color(tiny_skia::Color::WHITE);
                border.anti_alias = true;
                let stroke = Stroke {
                    width: 1.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
            }

            // Hex code label below swatch (round, don't truncate)
            let r8 = (color.r * 255.0 + 0.5) as u8;
            let g8 = (color.g * 255.0 + 0.5) as u8;
            let b8 = (color.b * 255.0 + 0.5) as u8;
            let hex = format!("#{:02x}{:02x}{:02x}", r8, g8, b8);

            let font_size = 12.0_f32;
            let text_width = measure_text_width(&hex, font_size);

            let pad_x = 4.0_f32;
            let pad_y = 2.0_f32;
            let pill_w = text_width + pad_x * 2.0;
            let pill_h = font_size + pad_y * 2.0;
            let pill_x = swatch_x;
            let pill_y = swatch_y + swatch_size + 2.0;

            if let Some(bg) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, 3.0) {
                let mut bg_paint = Paint::default();
                bg_paint.set_color(crate::theme::skia(crate::theme::bg_1(), 0.9));
                bg_paint.anti_alias = true;
                pixmap.fill_path(
                    &bg,
                    &bg_paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            use crate::tools::render_text_annotation;
            let text_pos = crate::geometry::Point::new(
                pill_x + pad_x,
                crate::ui::cap_centered_top(pill_y + pill_h / 2.0, font_size, 1.0),
            );
            let white = crate::theme::gcolor(crate::theme::text_normal(), 1.0);
            render_text_annotation(pixmap, &text_pos, &hex, &white, font_size);
        }
    }

    // 10. In-overlay toast notification
    state.clear_expired_toast();
    if let Some(ref msg) = state.toast_message {
        if state.preview_mode {
            let text: String = msg.chars().take(110).collect();
            let pos = crate::geometry::Point::new(8., state.screenshot.height as f32 + 50.);
            crate::tools::render_text_annotation(
                pixmap,
                &pos,
                &text,
                &crate::theme::gcolor(crate::theme::text_normal(), 1.),
                11.,
            );
            return;
        }
        let font_size = 14.0_f32;
        let text_width = measure_text_width(msg, font_size);

        let pad_x = 16.0_f32;
        let pad_y = 10.0_f32;
        let toast_w = text_width + pad_x * 2.0;
        let toast_h = font_size + pad_y * 2.0;
        let toast_x = (pixmap.width() as f32 - toast_w) / 2.0;
        let toast_y = pixmap.height() as f32 - 80.0;

        // Background pill
        if let Some(bg) = rounded_rect_path(toast_x, toast_y, toast_w, toast_h, 8.0) {
            let mut bg_paint = Paint::default();
            bg_paint.set_color(crate::theme::skia(crate::theme::bg_1(), 0.92));
            bg_paint.anti_alias = true;
            pixmap.fill_path(
                &bg,
                &bg_paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
            // Border
            let mut border = Paint::default();
            border.set_color(crate::theme::skia(crate::theme::accent(), 0.6));
            border.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&bg, &border, &stroke, Transform::identity(), None);
        }

        // Text
        use crate::tools::render_text_annotation;
        let text_pos = crate::geometry::Point::new(toast_x + pad_x, toast_y + pad_y);
        let text_color = crate::theme::gcolor(crate::theme::text_normal(), 1.0);
        render_text_annotation(pixmap, &text_pos, msg, &text_color, font_size);
    }
}

/// Build a rounded rectangle path with given corner radius.
fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    // Top edge (starting after top-left corner)
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    // Top-right corner
    pb.quad_to(x + w, y, x + w, y + r);
    // Right edge
    pb.line_to(x + w, y + h - r);
    // Bottom-right corner
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    // Bottom edge
    pb.line_to(x + r, y + h);
    // Bottom-left corner
    pb.quad_to(x, y + h, x, y + h - r);
    // Left edge
    pb.line_to(x, y + r);
    // Top-left corner
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Draw a dashed selection highlight rectangle around a selected annotation,
/// plus resize handles at the four corners.
fn render_selection_highlight(pixmap: &mut tiny_skia::Pixmap, x: f32, y: f32, w: f32, h: f32) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let highlight_color = crate::theme::skia(crate::theme::accent(), 0.8);
    if let Some(rect) = tiny_skia::Rect::from_xywh(x, y, w, h) {
        let path = PathBuilder::from_rect(rect);
        let mut paint = Paint::default();
        paint.set_color(highlight_color);
        paint.anti_alias = true;
        let stroke = Stroke {
            width: 1.5,
            dash: tiny_skia::StrokeDash::new(vec![4.0, 4.0], 0.0),
            ..Stroke::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    // Draw resize handles at the four corners
    let handles = [(x, y), (x + w, y), (x, y + h), (x + w, y + h)];
    for (hx, hy) in &handles {
        let hs = 4.0; // half-size
        if let Some(rect) = tiny_skia::Rect::from_xywh(hx - hs, hy - hs, hs * 2.0, hs * 2.0) {
            // White filled square
            let mut fill = Paint::default();
            fill.set_color(tiny_skia::Color::WHITE);
            fill.anti_alias = false;
            pixmap.fill_rect(rect, &fill, Transform::identity(), None);

            // Accent border to match the dashed selection border
            let path = PathBuilder::from_rect(rect);
            let mut border = Paint::default();
            border.set_color(highlight_color);
            border.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
        }
    }
}

fn render_toolbar(state: &mut OverlayState, selection: &Selection, pixmap: &mut tiny_skia::Pixmap) {
    let visible = &state.visible_buttons;
    let visible_count = visible.len();
    let toolbar = state.toolbar(selection);
    let presets = Color::presets();

    // --- Toolbar background: rounded rect with subtle border ---
    if let Some(bg_path) =
        rounded_rect_path(toolbar.x, toolbar.y, toolbar.width, toolbar.height, 8.0)
    {
        // Shadow (offset dark rect behind)
        if let Some(shadow_path) = rounded_rect_path(
            toolbar.x + 1.0,
            toolbar.y + 2.0,
            toolbar.width,
            toolbar.height,
            8.0,
        ) {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.4).unwrap());
            shadow_paint.anti_alias = true;
            pixmap.fill_path(
                &shadow_path,
                &shadow_paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        // Background fill
        let mut bg_paint = Paint::default();
        bg_paint.set_color(crate::theme::skia(crate::theme::bg_1(), 0.95));
        bg_paint.anti_alias = true;
        pixmap.fill_path(
            &bg_path,
            &bg_paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
        // Subtle border
        let mut border_paint = Paint::default();
        border_paint.set_color(crate::theme::skia(crate::theme::text_normal(), 0.12));
        border_paint.anti_alias = true;
        let border_stroke = Stroke {
            width: 1.0,
            ..Stroke::default()
        };
        pixmap.stroke_path(
            &bg_path,
            &border_paint,
            &border_stroke,
            Transform::identity(),
            None,
        );
    }

    // --- Separators between button groups (tools | colors | actions) ---
    // Find the visible positions where group boundaries occur (after last tool, after last color)
    let sep_color = crate::theme::skia(crate::theme::text_normal(), 0.15);
    // Separator after the last tool button (orig 0-13) before colors (orig 14)
    // and after the last color button (orig 18) before actions (orig 19)
    let sep_after_orig = [13usize, 18];
    for &boundary in &sep_after_orig {
        // Find the visible index of this boundary button
        if let Some(vis_idx) = visible.iter().position(|&orig| orig == boundary) {
            let (bx, _, bw, _) = toolbar.button_rect(vis_idx);
            let sep_x = bx + bw + TOOLBAR_PADDING / 2.0;
            let sep_y1 = toolbar.y + 8.0;
            let sep_y2 = toolbar.y + toolbar.height - 8.0;
            let mut pb = PathBuilder::new();
            pb.move_to(sep_x, sep_y1);
            pb.line_to(sep_x, sep_y2);
            if let Some(path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(sep_color);
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: 1.0,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        } else if boundary == 13 {
            // If button 13 is hidden, find the last visible tool button (0-13)
            // and draw separator after it
            if let Some(vis_idx) = visible.iter().rposition(|&orig| orig <= 13) {
                // Only draw if there's actually a color/action after it
                if vis_idx + 1 < visible_count {
                    let (bx, _, bw, _) = toolbar.button_rect(vis_idx);
                    let sep_x = bx + bw + TOOLBAR_PADDING / 2.0;
                    let sep_y1 = toolbar.y + 8.0;
                    let sep_y2 = toolbar.y + toolbar.height - 8.0;
                    let mut pb = PathBuilder::new();
                    pb.move_to(sep_x, sep_y1);
                    pb.line_to(sep_x, sep_y2);
                    if let Some(path) = pb.finish() {
                        let mut paint = Paint::default();
                        paint.set_color(sep_color);
                        paint.anti_alias = true;
                        let stroke = Stroke {
                            width: 1.0,
                            ..Stroke::default()
                        };
                        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                    }
                }
            }
        }
    }

    // --- Render each button ---
    for (vis_idx, &i) in visible.iter().enumerate() {
        let (bx, by, bw, bh) = toolbar.button_rect(vis_idx);
        let Some(def) = BUTTONS.get(i) else { continue };

        let is_active = match def.action {
            ButtonAction::Tool(kind) => state.active_tool == kind,
            ButtonAction::Color(idx) => idx < presets.len() && state.current_color == presets[idx],
            _ => false,
        };

        // Button background: rounded rect
        let btn_radius = 5.0;
        if let Some(btn_path) = rounded_rect_path(bx, by, bw, bh, btn_radius) {
            if is_active {
                // Active: filled highlight
                let mut fill = Paint::default();
                fill.set_color(crate::theme::skia(crate::theme::accent(), 0.3));
                fill.anti_alias = true;
                pixmap.fill_path(
                    &btn_path,
                    &fill,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
                // Active border glow
                let mut glow = Paint::default();
                glow.set_color(crate::theme::skia(crate::theme::accent(), 0.7));
                glow.anti_alias = true;
                let glow_stroke = Stroke {
                    width: 1.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&btn_path, &glow, &glow_stroke, Transform::identity(), None);
            } else {
                // Inactive: subtle hover-ready background
                let mut fill = Paint::default();
                fill.set_color(crate::theme::skia(crate::theme::text_normal(), 0.08));
                fill.anti_alias = true;
                pixmap.fill_path(
                    &btn_path,
                    &fill,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }

        // --- SVG icon rendering for tool buttons and action buttons ---
        if let Some(name) = def.icon {
            let icon_size = (bw - 12.0).max(12.0) as u32;
            let active_hex = crate::theme::hex(crate::theme::text_normal());
            let inactive_hex = crate::theme::hex(crate::theme::text_muted());
            let color_hex = if is_active {
                active_hex.as_str()
            } else {
                inactive_hex.as_str()
            };
            if let Some(icon_pixmap) = state.icon_cache.get_or_render(name, icon_size, color_hex) {
                let icon_x = (bx + (bw - icon_size as f32) / 2.0).round() as i32;
                let icon_y = (by + (bh - icon_size as f32) / 2.0).round() as i32;
                blend_pixmap(pixmap, icon_pixmap, icon_x, icon_y);
            }
        }

        if let ButtonAction::Color(idx) = def.action {
            // Color swatch: rounded filled rect with border
            if let Some(c) = presets.get(idx) {
                let (r, g, b) = (c.r, c.g, c.b);
                let inset = 6.0;
                if let Some(swatch_path) = rounded_rect_path(
                    bx + inset,
                    by + inset,
                    bw - inset * 2.0,
                    bh - inset * 2.0,
                    3.0,
                ) {
                    // Fill with color
                    let mut paint = Paint::default();
                    paint.set_color(
                        tiny_skia::Color::from_rgba(r, g, b, 1.0)
                            .unwrap_or(tiny_skia::Color::WHITE),
                    );
                    paint.anti_alias = true;
                    pixmap.fill_path(
                        &swatch_path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                    // Border (darker for light colors, lighter for dark)
                    let border_alpha = if r + g + b > 2.0 { 0.3 } else { 0.5 };
                    let mut border = Paint::default();
                    border.set_color(
                        tiny_skia::Color::from_rgba(1.0, 1.0, 1.0, border_alpha)
                            .unwrap_or(tiny_skia::Color::WHITE),
                    );
                    border.anti_alias = true;
                    let stroke = Stroke {
                        width: 1.0,
                        ..Stroke::default()
                    };
                    pixmap.stroke_path(&swatch_path, &border, &stroke, Transform::identity(), None);
                }
            }
        }
    }

    // --- Tooltip for hovered button ---
    let mouse = crate::geometry::Point::new(state.last_mouse_pos.x, state.last_mouse_pos.y);
    if let Some(vis_idx) = toolbar.hit_test_dynamic(mouse, visible_count) {
        let btn_idx = visible[vis_idx];
        let label = BUTTONS.get(btn_idx).map(|d| d.tooltip).unwrap_or("");

        if !label.is_empty() {
            let (btn_x, _btn_y, btn_w, _btn_h) = toolbar.button_rect(vis_idx);

            let font_size = 12.0;
            let text_width = measure_text_width(label, font_size);

            let pad_x = 6.0;
            let pad_y = 4.0;
            let tip_w = text_width + pad_x * 2.0;
            let tip_h = font_size + pad_y * 2.0;

            // Center tooltip above the button
            let tip_x = (btn_x + btn_w / 2.0 - tip_w / 2.0)
                .max(toolbar.x)
                .min(toolbar.x + toolbar.width - tip_w);
            let tip_y = toolbar.y - tip_h - 6.0;

            // Background pill
            if let Some(bg) = rounded_rect_path(tip_x, tip_y, tip_w, tip_h, 4.0) {
                let mut bg_paint = Paint::default();
                bg_paint.set_color(crate::theme::skia(crate::theme::bg_1(), 0.95));
                bg_paint.anti_alias = true;
                pixmap.fill_path(
                    &bg,
                    &bg_paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );

                // Border
                let mut border_paint = Paint::default();
                border_paint.set_color(crate::theme::skia(crate::theme::text_normal(), 0.2));
                border_paint.anti_alias = true;
                let border_stroke = Stroke {
                    width: 0.5,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &bg,
                    &border_paint,
                    &border_stroke,
                    Transform::identity(),
                    None,
                );
            }

            // Text
            use crate::tools::render_text_annotation;
            // Centre the caps; the line box would sit the text 2-3 pixels low.
            let text_pos = crate::geometry::Point::new(
                tip_x + pad_x,
                crate::ui::cap_centered_top(tip_y + tip_h / 2.0, font_size, 1.0),
            );
            let white = crate::theme::gcolor(crate::theme::text_normal(), 1.0);
            render_text_annotation(pixmap, &text_pos, label, &white, font_size);
        }
    }
}

use crate::overlay::toolbar::TOOLBAR_PADDING;

/// Render a small label showing the selection dimensions (e.g. "1920 × 1080")
/// near the bottom-right corner of the selection, flipping position if near screen edges.
fn render_size_label(
    pixmap: &mut tiny_skia::Pixmap,
    sel: &Selection,
    screen_w: f32,
    screen_h: f32,
) {
    let font_size = 14.0;

    let label = format!("{} \u{00D7} {}", sel.width as u32, sel.height as u32);
    let text_width = measure_text_width(&label, font_size);

    let pad_x: f32 = 8.0;
    let pad_y: f32 = 4.0;
    let pill_w = text_width + pad_x * 2.0;
    let pill_h = font_size + pad_y * 2.0;
    let margin = 4.0;

    // Default: bottom-right, offset outside selection
    let mut pill_x = sel.x + sel.width - pill_w + margin;
    let mut pill_y = sel.y + sel.height + margin;

    // Flip left if near right edge
    if pill_x + pill_w > screen_w {
        pill_x = sel.x - margin;
    }
    // Flip above if near bottom edge
    if pill_y + pill_h > screen_h {
        pill_y = sel.y - pill_h - margin;
    }

    // Clamp to screen
    if pill_x < 0.0 {
        pill_x = 0.0;
    }
    if pill_y < 0.0 {
        pill_y = 0.0;
    }

    // Draw background pill (dark semi-transparent rounded rect)
    if let Some(bg_path) = rounded_rect_path(pill_x, pill_y, pill_w, pill_h, 4.0) {
        let mut paint = Paint::default();
        paint.set_color(crate::theme::skia(crate::theme::bg_1(), 0.85));
        paint.anti_alias = true;
        pixmap.fill_path(
            &bg_path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    // Render text on top via the shared single-line text path
    use crate::tools::render_text_annotation;
    let text_pos = crate::geometry::Point::new(
        pill_x + pad_x,
        crate::ui::cap_centered_top(pill_y + pill_h / 2.0, font_size, 1.0),
    );
    let white = crate::theme::gcolor(crate::theme::text_normal(), 1.0);
    render_text_annotation(pixmap, &text_pos, &label, &white, font_size);
}
