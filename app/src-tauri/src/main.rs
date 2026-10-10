#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! ToolsBoXApp - desktop shell (WebView2) around the ToolsBoXMachine content.
//!
//! - The main window loads `index.html` from the embedded assets
//!     (frontendDist -> app/web/).
//! - Navigation between HTML files happens naturally inside the WebView.
//! - An initialization script (shim.js) is injected on every page load to
//!   handle: downloads, external links, clipboard and update checks.

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
            // The window is declared with "create": false in tauri.conf.json
            // so the initialization script can be attached to it.
            let window_config = app
                .handle()
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or("no window declared in app.windows (tauri.conf.json)")?;

            tauri::WebviewWindowBuilder::from_config(app.handle(), &window_config)?
                .initialization_script(shim::SHIM_JS)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ToolsBoXApp");
}
