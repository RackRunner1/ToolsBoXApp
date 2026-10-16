//! tbxa (alias tbxm) - terminal companion for ToolsBoXApp.
//!
//! Commands:
//!   tbxa update    - install the latest .rpm/.deb/.pkg.tar.zst from GitHub releases.
//!   tbxa uninstall - remove the ToolsBoXApp package.
//!   tbxa --version - print the installed version.
//!
//! `tbxm` is an alias: both names behave identically.
//!
//! The installed package name and version are discovered at runtime through
//! the RPM database (the package owning this very executable), the DPKG
//! database on Debian/Ubuntu, or pacman on Arch Linux, so no name is
//! hardcoded. External tools used:
//! `rpm`/`dpkg-query`/`pacman -Qo`, `curl`, `dnf`/`yum`, `apt-get` or
//! `pacman -U`/`pacman -R`, `sudo` (only when not running as root).

use std::process::Command;

const REPO: &str = "RackRunner1/ToolsBoXApp";
const API_URL: &str = "https://api.github.com/repos/RackRunner1/ToolsBoXApp/releases/latest";

fn main() {
    let prog = prog_name();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("update") => cmd_update(&prog),
        Some("uninstall") => cmd_uninstall(&prog),
        Some("-h") | Some("--help") | None => {
            print_help(&prog);
            0
        }
        Some("-V") | Some("--version") => cmd_version(&prog),
        Some(other) => {
            eprintln!("{prog}: unknown command '{other}'. See '{prog} --help'.");
            2
        }
    };
    std::process::exit(code);
}

/// Name under which the binary was invoked (`tbxa` by default, `tbxm` as alias).
/// Both names behave identically; this only affects help/error messages.
fn prog_name() -> String {
    let argv0 = std::env::args().next().unwrap_or_default();
    let base = argv0.rsplit('/').next().unwrap_or(&argv0);
    if base == "tbxm" {
        "tbxm".to_string()
    } else {
        "tbxa".to_string()
    }
}

fn print_help(prog: &str) {
    println!("{prog} - terminal companion for ToolsBoXApp");
    println!();
    println!("Usage: {prog} <command>");
    println!();
    println!("Commands:");
    println!("  update      Download the latest .rpm/.deb/.pkg.tar.zst from GitHub releases");
    println!("              and reinstall the app");
    println!("  uninstall   Remove the ToolsBoXApp package");
    println!("  --version   Print the installed version");
    println!("  --help      Print this help");
    println!();
    println!("Alias: tbxa and tbxm are interchangeable.");
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
    Pacman,
}

impl PkgKind {
    fn extension(self) -> &'static str {
        match self {
            PkgKind::Rpm => ".rpm",
            PkgKind::Deb => ".deb",
            PkgKind::Pacman => ".pkg.tar.zst",
        }
    }
}

/// Name of the installed package owning this executable via the RPM database,
/// e.g. "tools-bo-x-app".
fn installed_package_name_rpm() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|error| format!("cannot locate executable: {error}"))?;
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
    let exe = std::env::current_exe().map_err(|error| format!("cannot locate executable: {error}"))?;
    let output = run_capture("dpkg-query", &["--search", &exe.to_string_lossy()]).map_err(|_| {
        "tbxa does not seem to be installed through a DEB package.".to_string()
    })?;
    parse_dpkg_search_owner(&output)
        .ok_or_else(|| "could not determine the installed package name.".to_string())
}

/// Parses `pacman -Qo` output (`"/usr/bin/tbxa is owned by toolsboxapp 2026.10.16-1"`)
/// into the package name.
fn parse_pacman_query_owner(output: &str) -> Option<String> {
    let line = output.lines().next()?.trim();
    let marker = "is owned by";
    let pos = line.find(marker)?;
    let rest = line[pos + marker.len()..].trim();
    let name = rest.split_whitespace().next()?.trim();
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

/// Name of the installed package owning this executable via the pacman
/// database (Arch Linux), e.g. "toolsboxapp".
fn installed_package_name_pacman() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|error| format!("cannot locate executable: {error}"))?;
    let output = run_capture("pacman", &["-Qo", &exe.to_string_lossy()]).map_err(|_| {
        "tbxa does not seem to be installed through a pacman package.".to_string()
    })?;
    parse_pacman_query_owner(&output)
        .ok_or_else(|| "could not determine the installed package name.".to_string())
}

/// `(package_name, kind)`, trying RPM first, then DPKG, then pacman.
fn installed_package() -> Result<(String, PkgKind), String> {
    if let Ok(name) = installed_package_name_rpm() {
        return Ok((name, PkgKind::Rpm));
    }
    if let Ok(name) = installed_package_name_deb() {
        return Ok((name, PkgKind::Deb));
    }
    if let Ok(name) = installed_package_name_pacman() {
        return Ok((name, PkgKind::Pacman));
    }
    Err("tbxa does not seem to be installed through an RPM, DEB or pacman package.".to_string())
}

/// Strips a packaging revision suffix (`"2026.10.10-1"` -> `"2026.10.10"`),
/// matching the RPM `%{VERSION}` semantics used for comparisons.
/// Applies to Debian revisions and Arch `pkgrel` alike (neither version
/// core contains a `-`).
fn strip_release_suffix(version: &str) -> &str {
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
            let upstream = strip_release_suffix(full.trim()).to_string();
            if upstream.is_empty() {
                return Err("could not determine the installed package version.".to_string());
            }
            Ok(upstream)
        }
        PkgKind::Pacman => {
            // `pacman -Q` prints "<name> <pkgver>-<pkgrel>".
            let out = run_capture("pacman", &["-Q", package])?;
            let full = out.split_whitespace().nth(1).unwrap_or("").trim();
            let upstream = strip_release_suffix(full).to_string();
            if upstream.is_empty() {
                return Err("could not determine the installed package version.".to_string());
            }
            Ok(upstream)
        }
    }
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

/// Architecture markers for asset-name matching (same chip, different names
/// per ecosystem): `x86_64` also matches `amd64`/`x86-64`, `aarch64` also
/// matches `arm64`; any other architecture matches its own name.
fn arch_markers_for(arch: &str) -> Vec<String> {
    match arch {
        "x86_64" => vec![
            "x86_64".to_string(),
            "amd64".to_string(),
            "x86-64".to_string(),
        ],
        "aarch64" => vec!["aarch64".to_string(), "arm64".to_string()],
        _ => vec![arch.to_string()],
    }
}

/// `(tag_name, package_download_url)` from a GitHub "latest release" response
/// for the given asset extension (`".rpm"`, `".deb"` or `".pkg.tar.zst"`),
/// matching the given host architecture (see [`arch_markers_for`]).
///
/// Strict on architecture: when assets exist for the extension but none
/// matches the host, this returns an error instead of a foreign-arch
/// package (e.g. an `x86_64` build offered to an ARM machine).
fn parse_latest_release_for_arch(
    doc: &str,
    extension: &str,
    arch: &str,
) -> Result<(String, String), String> {
    let tag = json_string_value(doc, "tag_name")
        .filter(|tag| !tag.is_empty())
        .ok_or_else(|| "could not read tag_name from the GitHub response".to_string())?;

    let markers = arch_markers_for(arch);
    let mut candidates: Vec<String> = Vec::new();
    for object in json_asset_objects(doc) {
        let name = json_string_value(object, "name").unwrap_or_default();
        if !name.ends_with(extension) {
            continue;
        }
        let url = json_string_value(object, "browser_download_url").unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        candidates.push(name.clone());
        let lower = name.to_lowercase();
        if markers.iter().any(|marker| lower.contains(marker.as_str())) {
            return Ok((tag, url));
        }
    }

    if candidates.is_empty() {
        return Err(format!("no {extension} asset found in the latest release"));
    }
    Err(format!(
        "no {extension} asset matches this machine ({arch}) in the latest release ({}); refusing a foreign-arch package",
        candidates.join(", ")
    ))
}

/// `(tag_name, package_download_url)` from a GitHub "latest release" response
/// for the current host architecture.
fn parse_latest_release_for(doc: &str, extension: &str) -> Result<(String, String), String> {
    parse_latest_release_for_arch(doc, extension, std::env::consts::ARCH)
}

/// `(tag_name, rpm_download_url)` from a GitHub "latest release" response.
///
/// Shorthand for `parse_latest_release_for(doc, ".rpm")`; kept for tests.
#[cfg(test)]
fn parse_latest_release(doc: &str) -> Result<(String, String), String> {
    parse_latest_release_for(doc, ".rpm")
}

fn fetch_url(url: &str) -> Result<String, String> {
    if !has_tool("curl") {
        return Err(
            "curl is required but not installed (sudo dnf install curl / sudo apt install curl / sudo pacman -S curl)."
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
            "curl is required but not installed (sudo dnf install curl / sudo apt install curl / sudo pacman -S curl)."
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

/// `browser_download_url` of the `checksums.txt` release asset, if published.
/// The release pipeline (`daily-release.yml`) always uploads `checksums.txt`
/// (`sha256sum` of every package); that exact name wins, anything else
/// matching `sha256`/`checksum` is only a fallback.
fn find_checksums_url(doc: &str) -> Option<String> {
    let mut fallback: Option<String> = None;
    for object in json_asset_objects(doc) {
        let name = json_string_value(object, "name").unwrap_or_default();
        let url = json_string_value(object, "browser_download_url").unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        if name == "checksums.txt" {
            return Some(url);
        }
        let lower = name.to_lowercase();
        if (lower.contains("sha256") || lower.contains("checksum")) && fallback.is_none() {
            fallback = Some(url);
        }
    }
    fallback
}

/// SHA-256 hex digest of a file via `sha256sum` (else `shasum -a 256`).
fn sha256_of(path: &std::path::Path) -> Result<String, String> {
    let arg = path.to_string_lossy().to_string();
    let output = if has_tool("sha256sum") {
        run_capture("sha256sum", &[arg.as_str()])?
    }
    else if has_tool("shasum") {
        run_capture("shasum", &["-a", "256", arg.as_str()])?
    }
    else {
        return Err(
            "neither sha256sum nor shasum was found; cannot verify the download.".to_string(),
        );
    };
    output
        .split_whitespace()
        .next()
        .filter(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_string)
        .ok_or_else(|| "could not read the SHA-256 digest of the download.".to_string())
}

/// Checks `actual_hash` against the `checksums.txt` contents for `asset_name`.
/// `checksums.txt` uses `sha256sum` output format (`"<hash>  <basename>"`);
/// the filename must match exactly so a similarly-named file cannot stand in.
fn check_sha256(checksums: &str, asset_name: &str, actual_hash: &str) -> Result<(), String> {
    for line in checksums.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 {
            continue;
        }
        let hash = fields[0];
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let named = fields[1..].iter().any(|field| {
            let base = field.trim_start_matches('*');
            base == asset_name || base.rsplit('/').next().unwrap_or(base) == asset_name
        });
        if named {
            if hash.eq_ignore_ascii_case(actual_hash) {
                return Ok(());
            }
            return Err(format!(
                "SHA-256 mismatch for {asset_name} (release lists {hash}, download hashes to {actual_hash}). Aborting."
            ));
        }
    }
    Err(format!("no SHA-256 entry for {asset_name} in checksums.txt"))
}

/// Package manager used for installs, matching the native package format:
/// `dnf` else `yum` for RPM, `apt-get` for DEB, `pacman` for Arch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Manager {
    Dnf,
    Yum,
    AptGet,
    Pacman,
}

impl Manager {
    fn program(self) -> &'static str {
        match self {
            Manager::Dnf => "dnf",
            Manager::Yum => "yum",
            Manager::AptGet => "apt-get",
            Manager::Pacman => "pacman",
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
        PkgKind::Pacman => {
            if has_tool("pacman") {
                Ok(Manager::Pacman)
            }
            else {
                Err("pacman was not found. This tool targets Arch Linux for .pkg.tar.zst packages."
                    .to_string())
            }
        }
    }
}

/// Runs the package manager, with sudo unless already root.
/// `operation` is "install" (target is a package file) or "remove"
/// (target is a package name).
fn run_package_manager(manager: Manager, operation: &str, target: &str) -> Result<(), String> {
    let program = manager.program();
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
    if manager == Manager::Pacman {
        // pacman uses single-letter operations and --noconfirm instead of -y.
        let flag = match operation {
            "install" => "-U",
            "remove" => "-R",
            _ => operation,
        };
        command.arg(flag).arg("--noconfirm").arg(target);
    }
    else {
        // dnf/yum and apt-get share the "install"/"remove -y <target>" spelling.
        command.arg(operation).arg("-y").arg(target);
    }
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

fn cmd_version(prog: &str) -> i32 {
    match installed_package().and_then(|(package, kind)| installed_version(&package, kind)) {
        Ok(version) => {
            println!("{prog} (ToolsBoXApp {version})");
            0
        }
        Err(error) => {
            eprintln!("{prog}: {error}");
            1
        }
    }
}

fn cmd_update(prog: &str) -> i32 {
    let (package, kind) = match installed_package() {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };
    let current = match installed_version(&package, kind) {
        Ok(version) => version,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };

    println!("Installed version: {current}");
    println!("Checking {REPO} for the latest release...");

    let doc = match fetch_url(API_URL) {
        Ok(doc) => doc,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };
    let (tag, url) = match parse_latest_release_for(&doc, kind.extension()) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };
    println!("Latest release: {tag}");

    // No version comparison here on purpose: release tags are free-form
    // (they can be customized on manual runs), so `update` always installs
    // the latest published package instead of guessing from tag names.
    let manager = match package_manager(kind) {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };

    let default_name = match kind {
        PkgKind::Deb => "toolsboxapp-update.deb",
        PkgKind::Rpm => "toolsboxapp-update.rpm",
        PkgKind::Pacman => "toolsboxapp-update.pkg.tar.zst",
    };
    let file_name = url.rsplit('/').next().unwrap_or(default_name);
    let dest = std::env::temp_dir().join(format!(
        "tbxa-update-{}-{file_name}",
        std::process::id()
    ));
    println!("Downloading {}...", dest.to_string_lossy());
    if let Err(error) = download_file(&url, &dest) {
        eprintln!("{prog} update: {error}");
        return 1;
    }

    // HTTPS alone proves transport security, not artifact authenticity:
    // every release publishes checksums.txt, so refuse to install a
    // package that does not match its published SHA-256 hash.
    println!("Verifying SHA-256 against checksums.txt...");
    let sums_url = match find_checksums_url(&doc) {
        Some(url) => url,
        None => {
            eprintln!("{prog} update: no checksums.txt asset in release {tag}; refusing an unverifiable package.");
            return 1;
        }
    };
    let sums_dest = std::env::temp_dir().join(format!(
        "tbxa-update-{}-checksums.txt",
        std::process::id()
    ));
    if let Err(error) = download_file(&sums_url, &sums_dest) {
        eprintln!("{prog} update: {error}");
        return 1;
    }
    let sums = match std::fs::read_to_string(&sums_dest) {
        Ok(sums) => sums,
        Err(error) => {
            eprintln!("{prog} update: cannot read checksums.txt: {error}");
            return 1;
        }
    };
    let _ = std::fs::remove_file(&sums_dest);
    let actual = match sha256_of(&dest) {
        Ok(actual) => actual,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };
    if let Err(error) = check_sha256(&sums, file_name, &actual) {
        eprintln!("{prog} update: {error}");
        return 1;
    }
    println!("SHA-256 verified: {file_name} ({actual}).");

    println!(
        "Installing with {} (root privileges required)...",
        manager.program()
    );
    let result = run_package_manager(manager, "install", &dest.to_string_lossy());
    let _ = std::fs::remove_file(&dest);
    if let Err(error) = result {
        eprintln!("{prog} update: {error}");
        return 1;
    }

    println!("Update installed. Restart the app if it was running.");
    0
}

fn cmd_uninstall(prog: &str) -> i32 {
    let (package, kind) = match installed_package() {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("{prog} uninstall: {error}");
            return 1;
        }
    };
    let manager = match package_manager(kind) {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("{prog} uninstall: {error}");
            return 1;
        }
    };

    println!(
        "Removing package '{package}' with {} (root privileges required)...",
        manager.program()
    );
    if let Err(error) = run_package_manager(manager, "remove", &package) {
        eprintln!("{prog} uninstall: {error}");
        return 1;
    }

    println!("ToolsBoXApp uninstalled.");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn refuses_foreign_arch_asset() {
        // Only an ARM build published: an x86_64 machine must fail,
        // not silently install the wrong architecture.
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "other-1.aarch64.rpm", "browser_download_url": "https://example.com/other.rpm"}
        ]}"#;
        assert!(parse_latest_release_for_arch(doc, ".rpm", "x86_64").is_err());
        // And vice versa.
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "other-1.x86_64.rpm", "browser_download_url": "https://example.com/other.rpm"}
        ]}"#;
        assert!(parse_latest_release_for_arch(doc, ".rpm", "aarch64").is_err());
        // Each architecture matches its own naming.
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "toolsboxapp_1_amd64.deb", "browser_download_url": "https://example.com/amd64.deb"},
          {"name": "toolsboxapp_1_arm64.deb", "browser_download_url": "https://example.com/arm64.deb"}
        ]}"#;
        let (_, url) = parse_latest_release_for_arch(doc, ".deb", "aarch64").unwrap();
        assert_eq!(url, "https://example.com/arm64.deb");
        let (_, url) = parse_latest_release_for_arch(doc, ".deb", "x86_64").unwrap();
        assert_eq!(url, "https://example.com/amd64.deb");
        // Unknown architectures match their own name.
        let doc = r#"{"tag_name": "20261011", "assets": [
          {"name": "other-1.riscv64.rpm", "browser_download_url": "https://example.com/other.rpm"}
        ]}"#;
        assert!(parse_latest_release_for_arch(doc, ".rpm", "riscv64").is_ok());
        assert!(parse_latest_release_for_arch(doc, ".rpm", "x86_64").is_err());
    }

    #[test]
    fn finds_checksums_url() {
        assert_eq!(
            find_checksums_url(RELEASE_FIXTURE).as_deref(),
            Some("https://example.com/checksums.txt")
        );
        let doc = r#"{"assets": [
          {"name": "app.rpm", "browser_download_url": "https://example.com/app.rpm"}
        ]}"#;
        assert_eq!(find_checksums_url(doc), None);
    }

    #[test]
    fn verifies_sha256_entries() {
        let sums = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  app.rpm\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  other.deb\n";
        assert!(check_sha256(sums, "app.rpm", &"a".repeat(64)).is_ok());
        // Digests compare case-insensitively.
        assert!(check_sha256(sums, "other.deb", &"B".repeat(64)).is_ok());
        // Mismatch and missing entries fail closed.
        assert!(check_sha256(sums, "app.rpm", &"b".repeat(64)).is_err());
        assert!(check_sha256(sums, "missing.rpm", &"a".repeat(64)).is_err());
        // A similarly-named file cannot stand in for the real asset.
        let trap = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc  app.rpm.bak\n";
        assert!(check_sha256(trap, "app.rpm", &"c".repeat(64)).is_err());
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
    fn strips_release_suffix() {
        assert_eq!(strip_release_suffix("2026.10.11-1"), "2026.10.11");
        assert_eq!(strip_release_suffix("2026.10.11"), "2026.10.11");
        assert_eq!(strip_release_suffix("2026.10.10-1"), "2026.10.10");
    }

    #[test]
    fn parses_pacman_query_owner_output() {
        assert_eq!(
            parse_pacman_query_owner("/usr/bin/tbxa is owned by toolsboxapp 2026.10.16-1")
                .as_deref(),
            Some("toolsboxapp")
        );
        assert_eq!(
            parse_pacman_query_owner("/usr/bin/tbxm is owned by toolsboxapp 2026.10.16-1").as_deref(),
            Some("toolsboxapp")
        );
        assert_eq!(parse_pacman_query_owner(""), None);
        assert_eq!(parse_pacman_query_owner("error: No package owns /usr/bin/tbxa"), None);
    }

    #[test]
    fn parses_pacman_version_output() {
        // `pacman -Q` prints "<name> <pkgver>-<pkgrel>".
        let out = "toolsboxapp 2026.10.16-1";
        let full = out.split_whitespace().nth(1).unwrap_or("").trim();
        assert_eq!(strip_release_suffix(full), "2026.10.16");
    }

    #[test]
    fn pacman_asset_extension() {
        assert_eq!(PkgKind::Pacman.extension(), ".pkg.tar.zst");
        let doc = r#"{"tag_name": "20261016", "assets": [
          {"name": "ToolsBoXApp-2026.10.16-1.x86_64.rpm", "browser_download_url": "https://example.com/app.rpm"},
          {"name": "toolsboxapp_2026.10.16_amd64.deb", "browser_download_url": "https://example.com/app.deb"},
          {"name": "toolsboxapp-2026.10.16-1-x86_64.pkg.tar.zst", "browser_download_url": "https://example.com/app.pkg.tar.zst"}
        ]}"#;
        let (tag, url) = parse_latest_release_for(doc, ".pkg.tar.zst").unwrap();
        assert_eq!(tag, "20261016");
        assert_eq!(url, "https://example.com/app.pkg.tar.zst");
        // Other formats ignore the Arch asset and vice versa.
        let (tag, url) = parse_latest_release_for(doc, ".rpm").unwrap();
        assert_eq!(url, "https://example.com/app.rpm");
        assert_eq!(tag, "20261016");
        let arch_only = r#"{"tag_name": "20261016", "assets": [
          {"name": "toolsboxapp-2026.10.16-1-x86_64.pkg.tar.zst", "browser_download_url": "https://example.com/app.pkg.tar.zst"}
        ]}"#;
        assert!(parse_latest_release_for(arch_only, ".rpm").is_err());
        assert!(parse_latest_release_for(arch_only, ".deb").is_err());
    }
}
