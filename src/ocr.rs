//! OCR text extraction using the Windows built-in OCR engine.
//!
//! Uses PowerShell to access the WinRT `Windows.Media.Ocr` API, which is
//! available on Windows 10+ without any external dependencies.

/// Extract text from RGBA pixel data using Windows OCR.
///
/// Saves the image to a temporary PNG, invokes the WinRT OCR engine via
/// PowerShell, and returns the recognized text.
#[cfg(target_os = "windows")]
pub fn extract_text(pixels: &[u8], width: u32, height: u32) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let unique_name = format!(
        "snipping_tool_ul_ocr_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let temp_path = temp_dir.join(unique_name);

    let img =
        image::RgbaImage::from_raw(width, height, pixels.to_vec()).ok_or("Invalid image data")?;
    img.save(&temp_path).map_err(|e| e.to_string())?;

    // Guard ensures temp file is cleaned up even on panic
    struct TempFileGuard(std::path::PathBuf);
    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = TempFileGuard(temp_path.clone());

    let abs_path = temp_path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .to_string();

    // Strip the \\?\ prefix that canonicalize adds on Windows
    let abs_path = abs_path
        .strip_prefix(r"\\?\")
        .unwrap_or(&abs_path)
        .to_string();

    extract_text_powershell(&abs_path)
}

#[cfg(target_os = "windows")]
fn extract_text_powershell(image_path: &str) -> Result<String, String> {
    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime

# Helper to await WinRT async operations
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() |
    Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]

Function Await($WinRtTask, $ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $netTask = $asTask.Invoke($null, @($WinRtTask))
    $netTask.Wait(-1) | Out-Null
    $netTask.Result
}

# Load required WinRT types
[Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime] | Out-Null
[Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Ocr.OcrEngine, Windows.Foundation.UniversalApiContract, ContentType = WindowsRuntime] | Out-Null

$path = $env:SNIPPING_TOOL_UL_OCR_PATH
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($path)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) {
    Write-Error 'No OCR-capable language pack is installed. Add one under Windows Settings > Time & Language > Language.'
    exit 1
}
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
Write-Output $result.Text
"#;

    // CREATE_NO_WINDOW: without it a console window flashes on screen, since
    // the GUI-subsystem app has no console for powershell.exe to inherit.
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("powershell")
        .env("SNIPPING_TOOL_UL_OCR_PATH", image_path)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            Err("No text found in the selected region".to_string())
        } else {
            Ok(text)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("OCR failed: {stderr}"))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn extract_text(_pixels: &[u8], _width: u32, _height: u32) -> Result<String, String> {
    Err("OCR is only available on Windows 10+".to_string())
}
