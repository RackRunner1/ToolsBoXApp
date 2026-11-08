
mod download;
mod shim;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            download::save_download,
            download::reveal_file,
            download::open_external
        ])
        .setup(|app| {
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
