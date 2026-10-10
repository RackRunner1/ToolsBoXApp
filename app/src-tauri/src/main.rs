#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! ToolBoXApp — coquille desktop (WebView2) autour du contenu ToolsBoXMachine.
//!
//! - La fenêtre principale charge `index.html` depuis les assets embarqués
//!     (frontendDist -> app/web/).
//! - La navigation entre les fichiers HTML se fait naturellement dans le WebView.
//! - Un script d'initialisation (shim.js) est injecté à chaque chargement de
//!   page pour gérer : téléchargements, liens externes, presse-papier et la
//!   vérification des mises à jour.

mod download;
mod shim;
mod update;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            update::check_update,
            download::save_download,
            download::reveal_file,
            download::open_external
        ])
        .setup(|app| {
            // La fenêtre est déclarée avec "create": false dans tauri.conf.json
            // afin de pouvoir y attacher le script d'initialisation.
            let window_config = app
                .handle()
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or("aucune fenêtre déclarée dans app.windows (tauri.conf.json)")?;

            tauri::WebviewWindowBuilder::from_config(app.handle(), &window_config)?
                .initialization_script(shim::SHIM_JS)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de ToolBoXApp");
}
