//! Native OS color picker dialog.

#[cfg(target_os = "windows")]
pub fn pick_color(initial: &crate::geometry::Color) -> Option<crate::geometry::Color> {
    use windows::Win32::Foundation::*;
    use windows::Win32::UI::Controls::Dialogs::*;

    let initial_rgb = ((initial.r * 255.0 + 0.5) as u32)
        | (((initial.g * 255.0 + 0.5) as u32) << 8)
        | (((initial.b * 255.0 + 0.5) as u32) << 16);

    let mut custom_colors = [COLORREF(0); 16];

    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        rgbResult: COLORREF(initial_rgb),
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };

    unsafe {
        if ChooseColorW(&mut cc).as_bool() {
            let rgb = cc.rgbResult.0;
            let r = (rgb & 0xFF) as f32 / 255.0;
            let g = ((rgb >> 8) & 0xFF) as f32 / 255.0;
            let b = ((rgb >> 16) & 0xFF) as f32 / 255.0;
            Some(crate::geometry::Color::new(r, g, b, 1.0))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn pick_color(_initial: &crate::geometry::Color) -> Option<crate::geometry::Color> {
    // Native color picker not available on this platform
    None
}
