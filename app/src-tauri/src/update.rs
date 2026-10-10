//! In-app updates through the Tauri updater plugin.
//!
//! On startup, the JS shim calls `check_update`. The command asks the
//! updater (which reads `latest.json` from the `plugins.updater.endpoints`
//! configured in tauri.conf.json) whether a newer signed release exists.
//!
//! When the user confirms, `apply_update` downloads the update with progress
//! events (`tbx-update-progress`), verifies its signature, launches the NSIS
//! installer and exits: the installer finishes the job and restarts the app.
//!
//! Versioning scheme (set by the workflow):
//!   - app version : `YYYY.M.D` (semver, e.g. 2026.10.10)
//!   - release tag : `YYYYMMDD` (e.g. 20261010)
//! The semver comparison is done by the updater plugin itself.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// Fallback link (latest release page) shown when the updater has no
/// direct download URL to offer (manual install path).
const FALLBACK_RELEASES_PAGE: &str = "https://github.com/RackRunner1/ToolsBoXApp/releases/latest";

/// Result sent to the frontend to render the update banner.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// `true` only when a newer signed release is available.
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    /// Page of the latest release (manual fallback).
    pub release_url: String,
    /// Direct link to the update package, when announced.
    pub download_url: Option<String>,
    pub published_at: Option<String>,
    /// Set when the check failed (offline, updater not configured, ...).
    pub error: Option<String>,
}

/// Download progress sent to the frontend (`tbx-update-progress` event).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded: usize,
    total: Option<u64>,
}

fn no_update(current: String, error: Option<String>) -> UpdateInfo {
    UpdateInfo {
        available: false,
        current_version: current,
        latest_version: String::new(),
        release_url: FALLBACK_RELEASES_PAGE.to_string(),
        download_url: None,
        published_at: None,
        error,
    }
}

fn current_version(app: &AppHandle) -> String {
    let from_context = app.package_info().version.to_string();
    if from_context == "0.0.0" || from_context.is_empty() {
        env!("TBX_VERSION").to_string()
    }
    else {
        from_context
    }
}

/// Checks the update endpoint for a newer signed release.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = current_version(&app);

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => return Ok(no_update(current, Some(format!("updater unavailable: {error}")))),
    };

    // A failed check must never bother the user: report "no update"
    // together with the error message (logged by the frontend).
    match updater.check().await {
        Err(error) => Ok(no_update(current, Some(error.to_string()))),
        Ok(None) => Ok(no_update(current, None)),
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            current_version: update.current_version,
            latest_version: update.version,
            release_url: FALLBACK_RELEASES_PAGE.to_string(),
            download_url: Some(update.download_url.to_string()),
            published_at: update.date.map(|date| date.to_string()),
            error: None,
        }),
    }
}

/// Downloads, verifies and installs the pending update.
///
/// Progress is emitted as `tbx-update-progress` events
/// (`{ downloaded, total }`). On Windows the app exits once the installer
/// is launched, so the `Ok` value is rarely delivered to the frontend:
/// reaching 100% then losing the WebView means "it worked".
#[tauri::command]
pub async fn apply_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|error| format!("updater unavailable: {error}"))?;

    let Some(update) = updater.check().await.map_err(|error| error.to_string())?
    else {
        return Err("no update available".to_string());
    };

    update
        .download_and_install(
            {
                let app = app.clone();
                move |downloaded, total| {
                    let _ = app.emit(
                        "tbx-update-progress",
                        UpdateProgress { downloaded, total },
                    );
                }
            },
            || {},
        )
        .await
        .map_err(|error| error.to_string())
}
