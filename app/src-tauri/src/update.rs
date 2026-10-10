//! Update checks through the GitHub Releases API.
//!
//! On startup, the JS shim calls `check_update`. The command queries
//! `https://api.github.com/repos/<owner>/<repo>/releases/latest` and compares
//! the published tag with the current app version.
//!
//! Versioning scheme (set by the workflow):
//!   - app version    : `YYYY.M.D`   (semver, e.g. 2026.10.10)
//!   - release tag    : `YYYYMMDD`   (e.g. 20261010)
//! Both shapes are normalized to a (Y, M, D) triplet before comparing.

use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.github.com";
const FALLBACK_REPO: &str = "RackRunner1/ToolsBoXApp";
const TIMEOUT_SECS: u64 = 12;

/// Result sent to the frontend to render the update banner.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// `true` only when the published release is strictly newer.
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    /// Page of the latest release.
    pub release_url: String,
    /// Direct link to the x64 NSIS installer, when published.
    pub download_url: Option<String>,
    pub published_at: Option<String>,
    /// Set when the check failed (offline, rate limit, ...).
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

/// Derives `owner/repo` from `bundle.homepage` (tauri.conf.json).
fn repo_path(app: &tauri::AppHandle) -> String {
    let homepage = app
        .config()
        .bundle
        .homepage
        .clone()
        .unwrap_or_default();

    let trimmed = homepage.trim().trim_end_matches('/');
    let path = trimmed
        .rsplit("//")
        .next()
        .unwrap_or(trimmed)
        .trim_start_matches("www.");

    if path.contains('/') && !path.contains(char::is_whitespace) {
        path.to_string()
    }
    else {
        FALLBACK_REPO.to_string()
    }
}

/// Normalizes a version into a comparable triplet of numbers.
///
/// - `20261010`      -> `[2026, 10, 10]` (YYYYMMDD)
/// - `v2026.10.10`   -> `[2026, 10, 10]`
/// - `2026.1.1`      -> `[2026, 1, 1]`
/// - anything else   -> the numbers found, in order
fn version_tuple(version: &str) -> Vec<u64> {
    let s = version.trim().trim_start_matches(['v', 'V']);

    if s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()) {
        let compact: u64 = s.parse().unwrap_or(0);
        return vec![compact / 10_000, (compact / 100) % 100, compact % 100];
    }

    s.split(|c: char| c == '.' || c == '-' || c == '_' || c == '+' || c.is_whitespace())
        .filter(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

/// `true` when `candidate` is strictly newer than `current`.
fn is_newer(candidate: &str, current: &str) -> bool {
    let candidate = version_tuple(candidate);
    let current = version_tuple(current);

    for index in 0..candidate.len().max(current.len()) {
        let left = candidate.get(index).copied().unwrap_or(0);
        let right = current.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    false
}

/// Queries GitHub Releases and compares with the current version.
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    let fallback = env!("TBX_VERSION").to_string();
    let current = {
        let from_context = app.package_info().version.to_string();
        if from_context == "0.0.0" || from_context.is_empty() {
            fallback
        }
        else {
            from_context
        }
    };

    let repo = repo_path(&app);
    // Fallback link ("latest" page) used when the API is unreachable.
    // On success, the exact release URL (html_url) is used instead.
    let fallback_url = format!("https://github.com/{repo}/releases/latest");
    let api_url = format!("{API_BASE}/repos/{repo}/releases/latest");

    let client = reqwest::Client::builder()
        .user_agent(format!("ToolsBoXApp/{current} (+{fallback_url})"))
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|error| format!("HTTP client: {error}"))?;

    // A network failure must never bother the user: return a
    // "no update" result together with the error message.
    let response = match client.get(&api_url).send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(UpdateInfo {
                available: false,
                current_version: current,
                latest_version: String::new(),
                release_url: fallback_url,
                download_url: None,
                published_at: None,
                error: Some(format!("{error}")),
            });
        }
    };

    if !response.status().is_success() {
        return Ok(UpdateInfo {
            available: false,
            current_version: current,
            latest_version: String::new(),
            release_url: fallback_url,
            download_url: None,
            published_at: None,
            error: Some(format!("HTTP {}", response.status().as_u16())),
        });
    }

    let release: GitHubRelease = match response.json().await {
        Ok(release) => release,
        Err(error) => {
            return Ok(UpdateInfo {
                available: false,
                current_version: current,
                latest_version: String::new(),
                release_url: fallback_url,
                download_url: None,
                published_at: None,
                error: Some(format!("unreadable response: {error}")),
            });
        }
    };

    let latest_version = release
        .tag_name
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_string();

    let available = is_newer(&latest_version, &current);

    // The x64 NSIS installer is the preferred asset.
    let download_url = release
        .assets
        .iter()
        .find(|asset| asset.name.ends_with(".exe") && asset.name.contains("x64"))
        .or_else(|| release.assets.iter().find(|asset| asset.name.ends_with(".exe")))
        .map(|asset| asset.browser_download_url.clone());

    Ok(UpdateInfo {
        available,
        current_version: current,
        latest_version,
        release_url: release.html_url,
        download_url,
        published_at: release.published_at,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_date_tags_and_semver_versions() {
        assert!(is_newer("20261011", "2026.10.10"));
        assert!(is_newer("v2026.11.1", "2026.10.10"));
        assert!(!is_newer("20261010", "2026.10.10"));
        assert!(!is_newer("20260930", "2026.9.30"));
        assert!(is_newer("2026.10.9", "2026.9.30"));
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn parses_both_version_shapes() {
        assert_eq!(version_tuple("20261010"), vec![2026, 10, 10]);
        assert_eq!(version_tuple("v2026.10.10"), vec![2026, 10, 10]);
        assert_eq!(version_tuple("2026.1.1"), vec![2026, 1, 1]);
    }
}
