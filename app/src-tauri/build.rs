// Commandes exposees au frontend. Chaque commande genere automatiquement
// une permission `allow-<nom-snake>` referencee dans
// capabilities/default.json.
const APP_COMMANDS: &[&str] = &["check_update", "save_download", "reveal_file", "open_external"];

fn main() {
    // Version injectee par la CI (format AAAA.M.J, ex. 2026.10.10).
    // Sert de repli si la version du contexte Tauri n'est pas exploitable.
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
