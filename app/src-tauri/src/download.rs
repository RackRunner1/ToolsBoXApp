//! Utility commands called by the shim injected into HTML pages.
//!
//! - `save_download`: intercepts clicks on `<a download>` (pages generate
//!   blobs: QR codes, blurred images...) and saves them through the native
//!   dialog, as the WebView cannot handle downloads.
//! - `reveal_file`: opens the file manager on the folder containing a saved file.
//! - `open_external`: opens an http(s)/mailto link in the default browser.
//!
//! Linux-only build.

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// Header used by the shim to pass the file name (encoded).
const FILENAME_HEADER: &str = "x-tbx-filename";

/// Decodes a percent-encoded component (non-ASCII file names).
fn decode_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(byte) = hex.and_then(|hex| u8::from_str_radix(hex, 16).ok()) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| value.to_string())
}

/// Extension of the file name, when simple (used for the dialog filter).
fn file_extension(name: &str) -> Option<String> {
    let extension = std::path::Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())?;
    let extension = extension.trim().to_ascii_lowercase();
    if extension.is_empty() || extension.len() > 5 || !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
        None
    }
    else {
        Some(extension)
    }
}

/// Saves a file sent by the page (raw invoke body).
///
/// Returns `None` when the user cancels the dialog.
#[tauri::command]
pub async fn save_download(
    app: AppHandle,
    request: tauri::ipc::Request<'_>,
) -> Result<Option<String>, String> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err("expected raw body for save_download".to_string());
    };
    // `body()` returns a reference: copy the bytes for the background task.
    let data = data.clone();

    let file_name = request
        .headers()
        .get(FILENAME_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(decode_component)
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "download.bin".to_string());

    // Never block the WebView thread: dialog + disk write run on a worker task.
    let saved = tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>, String> {
        let mut builder = app
            .dialog()
            .file()
            .set_title("Save file")
            .set_file_name(file_name.clone());

        if let Some(extension) = file_extension(&file_name) {
            let label = format!("{} (*.{})", extension.to_uppercase(), extension);
            builder = builder.add_filter(label, &[extension.as_str()]);
        }
        let chosen = builder.add_filter("All files", &["*"]).blocking_save_file();

        let Some(chosen) = chosen else { return Ok(None) };
        let Some(path) = chosen.as_path().map(std::path::Path::to_path_buf) else {
            return Err("invalid location".to_string());
        };

        std::fs::write(&path, &data).map_err(|error| format!("cannot write file: {error}"))?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
    .map_err(|error| format!("save interrupted: {error}"))??;

    Ok(saved)
}

/// Opens the file manager on the folder containing the file.
#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.is_file() {
        return Err(format!("file not found: {path}"));
    }
    let folder = target
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));

    std::process::Command::new("xdg-open")
        .arg(&folder)
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "xdg-open not found.".to_string()
            }
            else {
                format!("cannot open file manager: {error}")
            }
        })?;
    Ok(())
}

/// Opens a URL in the default browser (allowed schemes only).
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let allowed = url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("mailto:");

    if !allowed {
        return Err(format!("scheme not allowed: {url}"));
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| format!("cannot open URL: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_encoded_names() {
        assert_eq!(decode_component("flout%C3%A9.png"), "flouté.png");
        assert_eq!(decode_component("qr%20code.png"), "qr code.png");
        assert_eq!(decode_component("simple.png"), "simple.png");
    }

    #[test]
    fn detects_simple_extensions() {
        assert_eq!(file_extension("image.PNG").as_deref(), Some("png"));
        assert_eq!(file_extension("archive.tar.gz").as_deref(), Some("gz"));
        assert_eq!(file_extension("no-extension"), None);
        assert_eq!(file_extension("too.longextension"), None);
    }
}
