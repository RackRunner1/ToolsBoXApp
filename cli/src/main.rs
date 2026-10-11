//! tbxa - terminal companion for ToolsBoXApp.
//!
//! Commands:
//!   tbxa update    - fetch the latest .rpm/.deb from GitHub releases and reinstall
//!                    when it is newer than the installed version.
//!   tbxa uninstall - remove the ToolsBoXApp package.
//!   tbxa --version - print the installed version.
//!
//! The installed package name and version are discovered at runtime through
//! the RPM database (the package owning this very executable) or the DPKG
//! database on Debian/Ubuntu, so no name is hardcoded. External tools used:
//! `rpm`/`dpkg-query`, `curl`, `dnf`/`yum` or `apt-get`, `sudo`
//! (only when not running as root).

use std::process::Command;

const REPO: &str = "RackRunner1/ToolsBoXApp";
const API_URL: &str = "https://api.github.com/repos/RackRunner1/ToolsBoXApp/releases/latest";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("update") => cmd_update(),
        Some("uninstall") => cmd_uninstall(),
        Some("-h") | Some("--help") | None => {
            print_help();
            0
        }
        Some("-V") | Some("--version") => cmd_version(),
        Some(other) => {
            eprintln!("tbxa: unknown command '{other}'. See 'tbxa --help'.");
            2
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!("tbxa - terminal companion for ToolsBoXApp");
    println!();
    println!("Usage: tbxa <command>");
    println!();
    println!("Commands:");
    println!("  update      Download the latest .rpm/.deb from GitHub releases and");
    println!("              reinstall the app when it is newer");
    println!("  uninstall   Remove the ToolsBoXApp package");
    println!("  --version   Print the installed version");
    println!("  --help      Print this help");
}

/// Runs a program and returns its trimmed stdout on success.
fn run_capture(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!("'{program}' not found. Is it installed?")
            }
            else {
                format!("cannot run '{program}': {error}")
            }
        })?;
    if !output.status.success() {
        let tail = String::from_utf8_lossy(&output.stderr);
        let tail = tail.lines().last().unwrap_or("unknown error");
        return Err(format!("'{program}' failed: {tail}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_root() -> bool {
    run_capture("id", &["-u"]).map(|out| out == "0").unwrap_or(false)
}

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Native package format owning this installation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PkgKind {
    Rpm,
    Deb,
}

impl PkgKind {
    fn extension(self) -> &'static str {
        match self {
            PkgKind::Rpm => ".rpm",
            PkgKind::Deb => ".deb",
        }
    }
}

/// Name of the installed package owning this executable via the RPM database,
/// e.g. "tools-bo-x-app".
fn installed_package_name_rpm() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|error| format!("cannot locate tbxa: {error}"))?;
    let name = run_capture(
        "rpm",
        &[
            "-qf",
            "--queryformat",
            "%{NAME}",
            &exe.to_string_lossy(),
        ],
    )
    .map_err(|_| {
        "tbxa does not seem to be installed through an RPM package.".to_string()
    })?;
    if name.is_empty() {
        return Err("could not determine the installed package name.".to_string());
    }
    Ok(name)
}

/// Parses `dpkg-query --search` output (`"pkg: /path"`) into the package name.
fn parse_dpkg_search_owner(output: &str) -> Option<String> {
    let first = output.lines().next()?.trim();
    let colon = first.find(':')?;
    let name = first[..colon].trim();
    // `dpkg-query -S` may list several packages ("a, b: /path"); keep the first.
    let name = name.split(',').next()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Name of the installed package owning this executable via the DPKG database
/// (Debian/Ubuntu), e.g. "toolsboxapp".
fn installed_package_name_deb() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|error| format!("cannot locate tbxa: {error}"))?;
    let output = run_capture("dpkg-query", &["--search", &exe.to_string_lossy()]).map_err(|_| {
        "tbxa does not seem to be installed through a DEB package.".to_string()
    })?;
    parse_dpkg_search_owner(&output)
        .ok_or_else(|| "could not determine the installed package name.".to_string())
}

/// `(package_name, kind)`, trying RPM first then DPKG.
fn installed_package() -> Result<(String, PkgKind), String> {
    if let Ok(name) = installed_package_name_rpm() {
        return Ok((name, PkgKind::Rpm));
    }
    if let Ok(name) = installed_package_name_deb() {
        return Ok((name, PkgKind::Deb));
    }
    Err("tbxa does not seem to be installed through an RPM or DEB package.".to_string())
}

/// Strips a Debian revision suffix (`"2026.10.10-1"` -> `"2026.10.10"`),
/// matching the RPM `%{VERSION}` semantics used for comparisons.
fn strip_debian_revision(version: &str) -> &str {
    match version.find('-') {
        Some(index) => &version[..index],
        None => version,
    }
}

/// Installed version of the package, e.g. "2026.10.10".
fn installed_version(package: &str, kind: PkgKind) -> Result<String, String> {
    match kind {
        PkgKind::Rpm => run_capture("rpm", &["-q", "--queryformat", "%{VERSION}", package]),
        PkgKind::Deb => {
            let full =
                run_capture("dpkg-query", &["--show", "--showformat=${Version}", package])?;
            let upstream = strip_debian_revision(full.trim()).to_string();
            if upstream.is_empty() {
                return Err("could not determine the installed package version.".to_string());
            }
            Ok(upstream)
        }
    }
}

/// Normalizes a version into a comparable triplet of numbers.
///
/// - `20261010`    -> `[2026, 10, 10]` (YYYYMMDD tags)
/// - `2026.10.10`  -> `[2026, 10, 10]` (RPM versions)
/// - anything else -> the numbers found, in order
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

/// Decodes a JSON string literal starting just after the opening quote.
/// Returns the decoded value and the byte index just past the closing quote.
fn decode_json_string(chars: &[u8], mut index: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    while index < chars.len() {
        match chars[index] {
            b'"' => return Some((out, index + 1)),
            b'\\' => {
                index += 1;
                if index >= chars.len() {
                    return None;
                }
                match chars[index] {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{0008}'),
                    b'f' => out.push('\u{000C}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        if index + 4 >= chars.len() {
                            return None;
                        }
                        let hex = std::str::from_utf8(&chars[index + 1..index + 5]).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        out.push(char::from_u32(code)?);
                        index += 4;
                    }
                    _ => return None,
                }
                index += 1;
            }
            _ => {
                // Multi-byte UTF-8 sequences are copied through from the
                // start of their leading byte.
                let start = index;
                index += 1;
                while index < chars.len() && (chars[index] & 0xC0) == 0x80 {
                    index += 1;
                }
                out.push_str(std::str::from_utf8(&chars[start..index]).ok()?);
            }
        }
    }
    None
}

/// Finds `"key"` in a JSON document and returns its string value.
/// Only handles string values; enough for the GitHub release fields used here.
fn json_string_value(doc: &str, key: &str) -> Option<String> {
    let bytes = doc.as_bytes();
    let needle = format!("\"{key}\"");
    let mut search_from = 0;
    while let Some(found) = find_bytes(&bytes[search_from..], needle.as_bytes()) {
        let mut index = search_from + found + needle.len();
        index = skip_json_whitespace(bytes, index);
        if bytes.get(index) != Some(&b':') {
            search_from = index;
            continue;
        }
        index = skip_json_whitespace(bytes, index + 1);
        if bytes.get(index) != Some(&b'"') {
            search_from = index;
            continue;
        }
        if let Some((value, _)) = decode_json_string(bytes, index + 1) {
            return Some(value);
        }
        search_from = index + 1;
    }
    None
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn skip_json_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
        index += 1;
    }
    index
}

/// Splits the top-level `{...}` objects of the `"assets"` array.
fn json_asset_objects(doc: &str) -> Vec<&str> {
    let bytes = doc.as_bytes();
    let mut objects = Vec::new();

    let key_pos = match find_bytes(bytes, b"\"assets\"") {
        Some(pos) => pos,
        None => return objects,
    };
    let mut index = skip_json_whitespace(bytes, key_pos + "\"assets\"".len());
    if bytes.get(index) != Some(&b':') {
        return objects;
    }
    index = skip_json_whitespace(bytes, index + 1);
    if bytes.get(index) != Some(&b'[') {
        return objects;
    }
    index += 1;

    loop {
        index = skip_json_whitespace(bytes, index);
        match bytes.get(index) {
            None | Some(b']') => break,
            Some(b',') => {
                index += 1;
                continue;
            }
            Some(b'{') => {
                let start = index;
                let mut depth = 0usize;
                let mut in_string = false;
                let mut escaped = false;
                while index < bytes.len() {
                    let byte = bytes[index];
                    if in_string {
                        if escaped {
                            escaped = false;
                        }
                        else if byte == b'\\' {
                            escaped = true;
                        }
                        else if byte == b'"' {
                            in_string = false;
                        }
                    }
                    else if byte == b'"' {
                        in_string = true;
                    }
                    else if byte == b'{' {
                        depth += 1;
                    }
                    else if byte == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            index += 1;
                            break;
                        }
                    }
                    index += 1;
                }
                if depth == 0 {
                    if let Ok(object) = std::str::from_utf8(&bytes[start..index]) {
                        objects.push(object);
                    }
                }
                else {
                    break;
                }
            }
            _ => {
                index += 1;
            }
        }
    }
    objects
}

/// `(tag_name, package_download_url)` from a GitHub "latest release" response
/// for the given asset extension (`".rpm"` or `".deb"`).
/// Prefers an asset with `x86_64`/`amd64` in its name.
fn parse_latest_release_for(doc: &str, extension: &str) -> Result<(String, String), String> {
    let tag = json_string_value(doc, "tag_name")
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "could not read tag_name from the GitHub response".to_string())?;

    let mut fallback: Option<String> = None;
    for object in json_asset_objects(doc) {
        let name = json_string_value(object, "name").unwrap_or_default();
        if !name.ends_with(extension) {
            continue;
        }
        let url = json_string_value(object, "browser_download_url").unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        if lower.contains("x86_64") || lower.contains("amd64") || lower.contains("x86-64") {
            return Ok((tag, url));
        }
        if fallback.is_none() {
            fallback = Some(url);
        }
    }

    fallback
        .map(|url| (tag.clone(), url))
        .ok_or_else(|| format!("no {extension} asset found in the latest release"))
}

/// `(tag_name, rpm_download_url)` from a GitHub "latest release" response.
/// Prefers an asset with `x86_64`/`amd64` in its name.
///
/// Shorthand for `parse_latest_release_for(doc, ".rpm")`; kept for tests.
#[cfg(test)]
fn parse_latest_release(doc: &str) -> Result<(String, String), String> {
    parse_latest_release_for(doc, ".rpm")
}

fn fetch_url(url: &str) -> Result<String, String> {
    if !has_tool("curl") {
        return Err(
            "curl is required but not installed (sudo dnf install curl / sudo apt install curl)."
                .to_string(),
        );
    }
    run_capture(
        "curl",
        &[
            "-fsSL",
            "--max-time",
            "60",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: tbxa",
            url,
        ],
    )
    .map_err(|error| format!("GitHub API request failed (network or rate limit?): {error}"))
}

fn download_file(url: &str, dest: &std::path::Path) -> Result<(), String> {
    if !has_tool("curl") {
        return Err(
            "curl is required but not installed (sudo dnf install curl / sudo apt install curl)."
                .to_string(),
        );
    }
    let status = Command::new("curl")
        .args([
            "-fSL",
            "--max-time",
            "600",
            "-o",
            &dest.to_string_lossy(),
            url,
        ])
        .status()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "curl is required but not installed (sudo dnf install curl / sudo apt install curl)."
                    .to_string()
            }
            else {
                format!("cannot run curl: {error}")
            }
        })?;
    if !status.success() {
        return Err("download failed (network issue?)".to_string());
    }
    Ok(())
}

/// Package manager used for installs, matching the native package format:
/// `dnf` else `yum` for RPM, `apt-get` for DEB.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Manager {
    Dnf,
    Yum,
    AptGet,
}

impl Manager {
    fn program(self) -> &'static str {
        match self {
            Manager::Dnf => "dnf",
            Manager::Yum => "yum",
            Manager::AptGet => "apt-get",
        }
    }
}

fn package_manager(kind: PkgKind) -> Result<Manager, String> {
    match kind {
        PkgKind::Rpm => {
            if has_tool("dnf") {
                Ok(Manager::Dnf)
            }
            else if has_tool("yum") {
                Ok(Manager::Yum)
            }
            else {
                Err("neither dnf nor yum was found. This tool targets RPM-based distributions."
                    .to_string())
            }
        }
        PkgKind::Deb => {
            if has_tool("apt-get") {
                Ok(Manager::AptGet)
            }
            else {
                Err("apt-get was not found. This tool targets Debian-based distributions for .deb packages."
                    .to_string())
            }
        }
    }
}

/// Runs the package manager, with sudo unless already root.
fn run_package_manager(manager: Manager, operation: &str, target: &str) -> Result<(), String> {
    let program = manager.program();
    // apt-get needs a distinct subcommand spelling (`install`/`remove`);
    // dnf/yum use the same words passed in by the caller.
    let operation = match (manager, operation) {
        (Manager::AptGet, "install") => "install",
        (Manager::AptGet, "remove") => "remove",
        _ => operation,
    };
    let mut command = if is_root() {
        Command::new(program)
    }
    else {
        if !has_tool("sudo") {
            return Err("root privileges are required: run as root or install sudo.".to_string());
        }
        let mut sudo = Command::new("sudo");
        sudo.arg(program);
        sudo
    };
    command.arg(operation).arg("-y").arg(target);
    let status = command.status().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "package manager tool not found.".to_string()
        }
        else {
            format!("cannot run package manager: {error}")
        }
    })?;
    if !status.success() {
        return Err(format!(
            "'{} {operation}' failed (see output above).",
            manager.program()
        ));
    }
    Ok(())
}

fn cmd_version() -> i32 {
    match installed_package().and_then(|(package, kind)| installed_version(&package, kind)) {
        Ok(version) => {
            println!("tbxa (ToolsBoXApp {version})");
            0
        }
        Err(error) => {
            eprintln!("tbxa: {error}");
            1
        }
    }
}

fn cmd_update() -> i32 {
    let (package, kind) = match installed_package() {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("tbxa update: {error}");
            return 1;
        }
    };
    let current = match installed_version(&package, kind) {
        Ok(version) => version,
        Err(error) => {
            eprintln!("tbxa update: {error}");
            return 1;
        }
    };

    println!("Installed version: {current}");
    println!("Checking {REPO} for a newer release...");

    let doc = match fetch_url(API_URL) {
        Ok(doc) => doc,
        Err(error) => {
            eprintln!("tbxa update: {error}");
            return 1;
        }
    };
    let (tag, url) = match parse_latest_release_for(&doc, kind.extension()) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("tbxa update: {error}");
            return 1;
        }
    };
    println!("Latest release: {tag}");

    if !is_newer(&tag, &current) {
        println!("Already up to date.");
        return 0;
    }

    let manager = match package_manager(kind) {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("tbxa update: {error}");
            return 1;
        }
    };

    let default_name = if kind == PkgKind::Deb {
        "toolsboxapp-update.deb"
    } else {
        "toolsboxapp-update.rpm"
    };
    let file_name = url.rsplit('/').next().unwrap_or(default_name);
    let dest = std::env::temp_dir().join(format!(
        "tbxa-update-{}-{file_name}",
        std::process::id()
    ));
    println!("Downloading {}...", dest.to_string_lossy());
    if let Err(error) = download_file(&url, &dest) {
        eprintln!("tbxa update: {error}");
        return 1;
    }

    println!(
        "Installing with {} (root privileges required)...",
        manager.program()
    );
    let result = run_package_manager(manager, "install", &dest.to_string_lossy());
    let _ = std::fs::remove_file(&dest);
    if let Err(error) = result {
        eprintln!("tbxa update: {error}");
        return 1;
    }

    println!("Update installed. Restart the app if it was running.");
    0
}

fn cmd_uninstall() -> i32 {
    let (package, kind) = match installed_package() {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("tbxa uninstall: {error}");
            return 1;
        }
    };
    let manager = match package_manager(kind) {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("tbxa uninstall: {error}");
            return 1;
        }
    };

    println!(
        "Removing package '{package}' with {} (root privileges required)...",
        manager.program()
    );
    if let Err(error) = run_package_manager(manager, "remove", &package) {
        eprintln!("tbxa uninstall: {error}");
        return 1;
    }

    println!("ToolsBoXApp uninstalled.");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_date_tags_and_rpm_versions() {
        assert!(is_newer("20261011", "2026.10.10"));
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

    #[test]
    fn extracts_json_string_values() {
        let doc = r#"{"tag_name": "20261011", "name": "x \"y\" \\ z", "n": 42, "u": "caf\u00e9 \u00e9"}"#;
        assert_eq!(json_string_value(doc, "tag_name").as_deref(), Some("20261011"));
        assert_eq!(json_string_value(doc, "name").as_deref(), Some("x \"y\" \\ z"));
        assert_eq!(json_string_value(doc, "missing"), None);
        assert_eq!(json_string_value(doc, "n"), None);
        assert_eq!(json_string_value(doc, "u").as_deref(), Some("café é"));
    }

    const RELEASE_FIXTURE: &str = r#"{
      "tag_name": "20261011",
      "assets": [
        {"name": "checksums.txt", "browser_download_url": "https://example.com/checksums.txt"},
        {"name": "ToolsBoXApp-2026.10.11-1.x86_64.rpm", "browser_download_url": "https://example.com/app.rpm"},
        {"name": "other-1.aarch64.rpm", "browser_download_url": "https://example.com/other.rpm"}
      ]
    }"#;

    #[test]
    fn parses_latest_release_prefers_x86_64() {
        let (tag, url) = parse_latest_release(RELEASE_FIXTURE).unwrap();
        assert_eq!(tag, "20261011");
        assert_eq!(url, "https://example.com/app.rpm");
    }

    #[test]
    fn parses_latest_release_falls_back_to_any_rpm() {
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "other-1.aarch64.rpm", "browser_download_url": "https://example.com/other.rpm"}
        ]}"#;
        let (tag, url) = parse_latest_release(doc).unwrap();
        assert_eq!(tag, "20261011");
        assert_eq!(url, "https://example.com/other.rpm");
    }

    #[test]
    fn parses_latest_release_rejects_missing_rpm() {
        let doc = r#"{"tag_name": "20261011", "assets": []}"#;
        assert!(parse_latest_release(doc).is_err());
        let bad = r#"{"nope": true}"#;
        assert!(parse_latest_release(bad).is_err());
    }

    #[test]
    fn parses_latest_release_prefers_deb_on_debian() {
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "ToolsBoXApp-2026.10.11-1.x86_64.rpm", "browser_download_url": "https://example.com/app.rpm"},
          {"name": "toolsboxapp_2026.10.11_amd64.deb", "browser_download_url": "https://example.com/app.deb"}
        ]}"#;
        let (tag, url) = parse_latest_release_for(doc, ".deb").unwrap();
        assert_eq!(tag, "20261011");
        assert_eq!(url, "https://example.com/app.deb");
        let (tag, url) = parse_latest_release_for(doc, ".rpm").unwrap();
        assert_eq!(tag, "20261011");
        assert_eq!(url, "https://example.com/app.rpm");
    }

    #[test]
    fn parses_latest_release_ignores_other_extension() {
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "toolsboxapp_2026.10.11_amd64.deb", "browser_download_url": "https://example.com/app.deb"}
        ]}"#;
        assert!(parse_latest_release_for(doc, ".rpm").is_err());
    }

    #[test]
    fn parses_dpkg_search_output() {
        assert_eq!(
            parse_dpkg_search_owner("toolsboxapp: /usr/bin/tbxa").as_deref(),
            Some("toolsboxapp")
        );
        assert_eq!(
            parse_dpkg_search_owner("a, b: /usr/bin/tbxa").as_deref(),
            Some("a")
        );
        assert_eq!(parse_dpkg_search_owner(""), None);
        assert_eq!(parse_dpkg_search_owner("no-colon-here"), None);
    }

    #[test]
    fn strips_debian_revision() {
        assert_eq!(strip_debian_revision("2026.10.11-1"), "2026.10.11");
        assert_eq!(strip_debian_revision("2026.10.11"), "2026.10.11");
        assert!(is_newer("20261011", strip_debian_revision("2026.10.10-1")));
        assert!(!is_newer("20261010", strip_debian_revision("2026.10.10-1")));
    }
}
