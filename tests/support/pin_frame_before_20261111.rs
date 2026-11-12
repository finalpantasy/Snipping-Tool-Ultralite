// The 0.3.27 pin frame build, kept to prove the row-wise version is identical.
#[allow(dead_code)]
fn pin_frame_before(pixels: &[u8], img_w: u32, img_h: u32, accent: (u8, u8, u8)) -> Vec<u8> {
    let border = 3u32;
    let shadow = 2u32;
    let total_w = img_w + border * 2 + shadow;
    let total_h = img_h + border * 2 + shadow;
        // Build the framed pixel buffer
        let mut framed = vec![0u8; (total_w * total_h * 4) as usize];

        // Shadow fill (dark in both themes, offset bottom-right)
        for y in shadow..total_h {
            for x in shadow..total_w {
                let i = ((y * total_w + x) * 4) as usize;
                if i + 3 < framed.len() {
                    framed[i] = 0;
                    framed[i + 1] = 0;
                    framed[i + 2] = 0;
                    framed[i + 3] = 100; // semi-transparent (visible in the saved PNG)
                }
            }
        }

        // Border fill (theme accent)
        let (bd_r, bd_g, bd_b) = accent;
        for y in 0..total_h - shadow {
            for x in 0..total_w - shadow {
                let i = ((y * total_w + x) * 4) as usize;
                if i + 3 < framed.len() {
                    framed[i] = bd_r; // theme accent R
                    framed[i + 1] = bd_g;
                    framed[i + 2] = bd_b;
                    framed[i + 3] = 255;
                }
            }
        }

        // Copy the actual image inside the border
        for y in 0..img_h {
            for x in 0..img_w {
                let src = ((y * img_w + x) * 4) as usize;
                let dst = (((y + border) * total_w + x + border) * 4) as usize;
                if src + 3 < pixels.len() && dst + 3 < framed.len() {
                    framed[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
                }
            }
        }

        framed
}
