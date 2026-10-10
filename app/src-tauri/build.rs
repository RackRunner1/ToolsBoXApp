// Commandes exposees au frontend. Chaque commande genere une permission
// `allow-<nom-kebab>` referencee dans capabilities/default.json.
const APP_COMMANDS: &[&str] = &["check_update", "save_download", "reveal_file", "open_external"];

fn main() {
    // Version injectee par la CI (format YYYY.M.D, ex. 2026.10.10).
    // Sert de repli si la config Tauri n'est pas lue correctement.
    let fallback = std::env::var("TBX_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=TBX_VERSION={fallback}");
    println!("cargo:rerun-if-env-changed=TBX_VERSION");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=../web/index.html");

    tauri_build::Builder::new()
        .commands(APP_COMMANDS)
        .build()
}
