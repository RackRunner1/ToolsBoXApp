// Commands exposed to the frontend. Each command auto-generates an
// `allow-<snake-name>` permission referenced in capabilities/default.json.
const APP_COMMANDS: &[&str] = &["save_download", "reveal_file", "open_external"];

fn main() {
    // Version injected by CI (YYYY.M.D shape, e.g. 2026.10.10).
    // Fallback when the Tauri context version is not usable.
    let fallback = std::env::var("TBX_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=TBX_VERSION={fallback}");
    println!("cargo:rerun-if-env-changed=TBX_VERSION");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=../web/index.html");

    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(APP_COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
