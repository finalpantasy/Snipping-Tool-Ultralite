use snipping_tool_ul::geometry::{Color, Point};
use tiny_skia::Pixmap;
pub fn reference_text(
    pixmap: &mut Pixmap,
    position: &Point,
    text: &str,
    color: &Color,
    font_size: f32,
) {
    let font = snipping_tool_ul::font::FONT.for_text(text);

    let pw = pixmap.width() as i32;
    let ph = pixmap.height() as i32;

    let sr = (color.r * 255.0 + 0.5) as u8;
    let sg = (color.g * 255.0 + 0.5) as u8;
    let sb = (color.b * 255.0 + 0.5) as u8;
    let sa = color.a;

    let mut cursor_x = position.x;
    // Treat click position as the TOP of the text line.
    // Compute baseline from font metrics: baseline = top + ascent
    let line_metrics = font.horizontal_line_metrics(font_size);
    let ascent = line_metrics.map(|m| m.ascent).unwrap_or(font_size * 0.8);
    let baseline_y = position.y + ascent;

    let pixels = pixmap.data_mut();

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        if metrics.width == 0 || metrics.height == 0 {
            cursor_x += metrics.advance_width;
            continue;
        }

        // fontdue: ymin is the distance from baseline to the bottom of the glyph bitmap
        // glyph top = baseline - (height + ymin) ... but we use the standard formula:
        // glyph_y = baseline - glyph_top_offset, where glyph_top_offset = height + ymin
        let glyph_x = cursor_x.round() as i32 + metrics.xmin;
        let glyph_y = baseline_y.round() as i32 - metrics.height as i32 - metrics.ymin;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let px_x = glyph_x + col as i32;
                let px_y = glyph_y + row as i32;

                if px_x < 0 || px_x >= pw || px_y < 0 || px_y >= ph {
                    continue;
                }

                let coverage = bitmap[row * metrics.width + col] as f32 / 255.0;
                let alpha = coverage * sa;
                if alpha < 1.0 / 255.0 {
                    continue;
                }

                let idx = ((px_y as u32 * pw as u32 + px_x as u32) * 4) as usize;

                // Alpha-blend: source (straight) over destination (premultiplied).
                // Keep everything in f32 to avoid precision loss at low alpha.
                let src_r_f = sr as f32 * alpha;
                let src_g_f = sg as f32 * alpha;
                let src_b_f = sb as f32 * alpha;
                let src_a_f = alpha * 255.0;

                let inv_alpha = 1.0 - alpha;
                let out_r = src_r_f + pixels[idx] as f32 * inv_alpha;
                let out_g = src_g_f + pixels[idx + 1] as f32 * inv_alpha;
                let out_b = src_b_f + pixels[idx + 2] as f32 * inv_alpha;
                let out_a = src_a_f + pixels[idx + 3] as f32 * inv_alpha;

                pixels[idx] = (out_r + 0.5).min(255.0) as u8;
                pixels[idx + 1] = (out_g + 0.5).min(255.0) as u8;
                pixels[idx + 2] = (out_b + 0.5).min(255.0) as u8;
                pixels[idx + 3] = (out_a + 0.5).min(255.0) as u8;
            }
        }

        cursor_x += metrics.advance_width;
    }
}

use snipping_tool_ul::tools::Annotation;
use tiny_skia::{Paint, PathBuilder, Stroke, Transform};
pub fn reference_marker(annotation: &Annotation, pixmap: &mut Pixmap) {
    match annotation {
        Annotation::StepMarker {
            position,
            number,
            color,
            size,
        } => {
            let radius = size / 2.0;

            // Draw filled circle using bezier approximation
            let k: f32 = 0.5522848;
            let kr = radius * k;
            let mut pb = PathBuilder::new();
            pb.move_to(position.x + radius, position.y);
            pb.cubic_to(
                position.x + radius,
                position.y + kr,
                position.x + kr,
                position.y + radius,
                position.x,
                position.y + radius,
            );
            pb.cubic_to(
                position.x - kr,
                position.y + radius,
                position.x - radius,
                position.y + kr,
                position.x - radius,
                position.y,
            );
            pb.cubic_to(
                position.x - radius,
                position.y - kr,
                position.x - kr,
                position.y - radius,
                position.x,
                position.y - radius,
            );
            pb.cubic_to(
                position.x + kr,
                position.y - radius,
                position.x + radius,
                position.y - kr,
                position.x + radius,
                position.y,
            );
            pb.close();

            if let Some(path) = pb.finish() {
                // Fill with annotation color
                let mut paint = Paint::default();
                paint.set_color((*color).into());
                paint.anti_alias = true;
                pixmap.fill_path(
                    &path,
                    &paint,
                    tiny_skia::FillRule::Winding,
                    Transform::identity(),
                    None,
                );

                // White border
                let mut border = Paint::default();
                border.set_color(tiny_skia::Color::WHITE);
                border.anti_alias = true;
                let stroke = Stroke {
                    width: 2.0,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &border, &stroke, Transform::identity(), None);
            }

            // Black text for consistent readability on all circle colors
            let text_color = Color::new(0.0, 0.0, 0.0, 1.0);
            let num_str = number.to_string();
            let fs = size * 0.55;

            // Measure actual text dimensions using fontdue for precise centering
            let font = &*snipping_tool_ul::font::FONT;

            // Measure total advance width
            let total_width: f32 = num_str
                .chars()
                .map(|ch| {
                    let (metrics, _) = font.rasterize(ch, fs);
                    metrics.advance_width
                })
                .sum();

            // Get line metrics for vertical centering
            let line_metrics = font.horizontal_line_metrics(fs);
            let ascent = line_metrics.map(|m| m.ascent).unwrap_or(fs * 0.8);
            let descent = line_metrics.map(|m| m.descent.abs()).unwrap_or(fs * 0.2);
            let text_height = ascent + descent;

            // Center horizontally and vertically in the circle
            // render_text_annotation treats position as the TOP of the text
            let text_x = position.x - total_width * 0.5;
            let text_y = position.y - text_height * 0.5;
            reference_text(
                pixmap,
                &Point::new(text_x, text_y),
                &num_str,
                &text_color,
                fs,
            );
        }
        _ => unreachable!(),
    }
}
