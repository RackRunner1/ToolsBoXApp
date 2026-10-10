//! Commandes utilitaires appelées par le shim injecté dans les pages HTML.
//!
//! - `save_download` : intercepte les clics sur `<a download>` (les pages
//!   génèrent des blobs : QR codes, images floutées…) et les enregistre via
//!   la boîte de dialogue native, le WebView ne gérant pas les téléchargements.
//! - `reveal_file`   : ouvre l'explorateur sur un fichier enregistré.
//! - `open_external` : ouvre un lien http(s)/mailto dans le navigateur système.

use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// En-tête utilisé par le shim pour transmettre le nom de fichier (encodé).
const FILENAME_HEADER: &str = "x-tbx-filename";

/// Décode un composant percent-encodé (noms de fichiers non ASCII).
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

/// Extension du nom de fichier, si elle est simple (utilisée pour le filtre).
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

/// Enregistre un fichier envoyé par la page (corps brut de l'invoke).
///
/// Retourne `None` si l'utilisateur annule la boîte de dialogue.
#[tauri::command]
pub async fn save_download(
    app: AppHandle,
    request: tauri::ipc::Request<'_>,
) -> Result<Option<String>, String> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err("contenu brut attendu pour save_download".to_string());
    };
    // `body()` rend une référence : on copie les octets pour les donner à la tâche.
    let data = data.clone();

    let file_name = request
        .headers()
        .get(FILENAME_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(decode_component)
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "telechargement.bin".to_string());

    // Ne jamais bloquer le thread du WebView : boîte de dialogue + écriture
    // disque dans une tâche dédiée.
    let saved = tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>, String> {
        let mut builder = app
            .dialog()
            .file()
            .set_title("Enregistrer le fichier")
            .set_file_name(file_name.clone());

        if let Some(extension) = file_extension(&file_name) {
            let label = format!("{} (*.{})", extension.to_uppercase(), extension);
            builder = builder.add_filter(label, &[extension.as_str()]);
        }
        let chosen = builder.add_filter("Tous les fichiers", &["*"]).blocking_save_file();

        let Some(chosen) = chosen else { return Ok(None) };
        let Some(path) = chosen.as_path().map(std::path::Path::to_path_buf) else {
            return Err("emplacement invalide".to_string());
        };

        std::fs::write(&path, &data).map_err(|error| format!("écriture impossible : {error}"))?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
    .map_err(|error| format!("enregistrement interrompu : {error}"))??;

    Ok(saved)
}

/// Ouvre l'explorateur Windows avec le fichier sélectionné.
#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.is_file() {
        return Err(format!("fichier introuvable : {path}"));
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS | CREATE_NO_WINDOW : pas de console qui clignote.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{path}\""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| format!("explorer : {error}"))?;
    }

    #[cfg(not(windows))]
    {
        let _ = &target;
        return Err("reveal_file n'est disponible que sur Windows".to_string());
    }

    Ok(())
}

/// Ouvre une URL dans le navigateur par défaut (schémas autorisés uniquement).
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let allowed = url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("mailto:");

    if !allowed {
        return Err(format!("schéma non autorisé : {url}"));
    }

    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| format!("ouverture impossible : {error}"))
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
        assert_eq!(file_extension("sans-extension"), None);
        assert_eq!(file_extension("trop.longueextension"), None);
    }
}
