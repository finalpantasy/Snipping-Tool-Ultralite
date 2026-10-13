fn reference_pixelation(
    p: &mut Pixmap,
    a: &Annotation,
    screen: &crate::capture::CapturedScreen,
    scale: f32,
    ox: f32,
    oy: f32,
    cache: &mut PixelationCache,
) {
    let Annotation::Pixelate {
        top_left,
        size,
        block_size,
    } = a
    else {
        return;
    };
    let x = top_left.x.max(0.) as u32;
    let y = top_left.y.max(0.) as u32;
    let w = ((size.width + top_left.x.min(0.)).max(0.) as u32).min(screen.width.saturating_sub(x));
    let h =
        ((size.height + top_left.y.min(0.)).max(0.) as u32).min(screen.height.saturating_sub(y));
    if w == 0 || h == 0 || !scale.is_finite() || scale <= 0. {
        return;
    }
    let key = PixelationKey {
        source: screen.pixels.as_ptr() as usize,
        width: screen.width,
        height: screen.height,
        x,
        y,
        w,
        h,
        block: (*block_size).max(1) as u32,
    };
    let left = x as f32 * scale + ox;
    let top = y as f32 * scale + oy;
    let right = (x + w) as f32 * scale + ox;
    let bottom = (y + h) as f32 * scale + oy;
    let x0 = left.floor().clamp(0., p.width() as f32) as u32;
    let x1 = right.ceil().clamp(0., p.width() as f32) as u32;
    let y0 = top.floor().clamp(0., p.height() as f32) as u32;
    let y1 = bottom.ceil().clamp(0., p.height() as f32) as u32;
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let grid = cache.grid(screen, key);
    let columns = w.div_ceil(key.block);
    let rows = h.div_ceil(key.block);
    // Sample only visible output pixels. Tiny blocks that exceed the cache budget
    // skip grid allocation, instead of drawing millions of subpixel rectangles.
    let xs: Vec<_> = (x0..x1)
        .map(|xx| {
            let source = ((xx as f32 + 0.5 - left) / scale).clamp(0., w as f32 - f32::EPSILON);
            let bx = ((source / key.block as f32) as u32).min(columns - 1);
            let coverage = ((xx + 1) as f32).min(right) - (xx as f32).max(left);
            (bx, coverage.clamp(0., 1.))
        })
        .collect();
    let stride = p.width() as usize * 4;
    let pixels = p.data_mut();
    for yy in y0..y1 {
        let source = ((yy as f32 + 0.5 - top) / scale).clamp(0., h as f32 - f32::EPSILON);
        let by = ((source / key.block as f32) as u32).min(rows - 1);
        let cy = (((yy + 1) as f32).min(bottom) - (yy as f32).max(top)).clamp(0., 1.);
        let mut previous = None;
        let mut color = [0; 3];
        for (dx, &(bx, cx)) in xs.iter().enumerate() {
            if previous != Some(bx) {
                // This reference checks opaque images against the previous RGB path.
                let rgba = grid
                    .map(|g| g.colors[(by * g.columns + bx) as usize])
                    .unwrap_or_else(|| block_color(screen, key, bx, by));
                color.copy_from_slice(&rgba[..3]);
                previous = Some(bx);
            }
            let i = yy as usize * stride + (x0 as usize + dx) * 4;
            let alpha = (cx * cy * 255.).round() as u32;
            if alpha == 255 {
                pixels[i..i + 3].copy_from_slice(&color);
                pixels[i + 3] = 255;
            } else {
                for c in 0..3 {
                    pixels[i + c] =
                        ((color[c] as u32 * alpha + pixels[i + c] as u32 * (255 - alpha) + 127)
                            / 255) as u8;
                }
                pixels[i + 3] = (alpha + (pixels[i + 3] as u32 * (255 - alpha) + 127) / 255) as u8;
            }
        }
    }
}
