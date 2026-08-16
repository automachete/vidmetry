#[cfg(windows)]
use windows_sys::Win32::Graphics::Dwm::DwmGetColorizationColor;

fn argb_to_rgb_hex(argb: u32) -> String {
    format!("#{:06X}", argb & 0x00FF_FFFF)
}

#[tauri::command]
pub fn system_accent_color() -> Result<String, String> {
    #[cfg(windows)]
    {
        let mut color = 0_u32;
        let mut opaque_blend = 0_i32;
        // SAFETY: Both pointers reference initialized, writable values for the duration of the call.
        let result = unsafe { DwmGetColorizationColor(&mut color, &mut opaque_blend) };
        if result < 0 {
            return Err(format!(
                "Unable to read the Windows accent color (HRESULT {result:#X})"
            ));
        }
        Ok(argb_to_rgb_hex(color))
    }

    #[cfg(not(windows))]
    {
        Ok("#0078D4".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::argb_to_rgb_hex;

    #[test]
    fn converts_dwm_argb_to_css_rgb() {
        assert_eq!(argb_to_rgb_hex(0xC4_00_78_D4), "#0078D4");
        assert_eq!(argb_to_rgb_hex(0xFF_FF_8C_00), "#FF8C00");
    }
}
