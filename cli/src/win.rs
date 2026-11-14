use std::path::Path;
use std::process::Command;

use super::{
    check_sha256, download_file, fail, fetch_url, find_checksums_url, is_newer,
    parse_latest_release_for, print_banner, release_version, run_capture, Progress, API_URL,
    TEXT_INDENT,
};

const UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ToolsBoXApp";
const INSTALLER_EXTENSION: &str = ".exe";
const SILENT_FLAG: &str = "/S";

fn parse_reg_value(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(name)?;
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        let (kind, value) = rest.trim_start().split_once(char::is_whitespace)?;
        if !kind.starts_with("REG_") {
            return None;
        }
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn registry_value(name: &str) -> Result<String, String> {
    let output = Command::new("reg")
        .args(["query", UNINSTALL_KEY, "/v", name])
        .output()
        .map_err(|error| format!("cannot run reg: {error}"))?;
    if !output.status.success() {
        return Err("ToolsBoXApp does not seem to be installed for this user.".to_string());
    }
    parse_reg_value(&String::from_utf8_lossy(&output.stdout), name)
        .ok_or_else(|| format!("cannot read {name} from the uninstall entry."))
}

fn installed_version() -> Result<String, String> {
    registry_value("DisplayVersion")
}

fn uninstaller() -> Result<String, String> {
    let raw = registry_value("UninstallString")?;
    Ok(raw.trim().trim_matches('"').to_string())
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let arg = path.to_string_lossy().to_string();
    let output = run_capture("certutil", &["-hashfile", &arg, "SHA256"])?;
    output
        .lines()
        .map(|line| line.replace(' ', ""))
        .find(|line| line.len() == 64 && line.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| "could not read the SHA-256 digest of the download.".to_string())
}

fn run_after_exit(program: &Path, args: &str) -> Result<(), String> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    let line = format!("/C ping -n 3 127.0.0.1 >NUL & \"{}\" {args}", program.display());
    let mut command = Command::new("cmd");
    #[cfg(windows)]
    command.raw_arg(line);
    #[cfg(not(windows))]
    command.arg(line);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot start the installer: {error}"))
}

fn fetch_latest(prog: &str, op: &str) -> Result<(String, String, String), String> {
    let doc = fetch_url(API_URL).map_err(|error| format!("{prog} {op}: {error}"))?;
    let (tag, url) = parse_latest_release_for(&doc, INSTALLER_EXTENSION)
        .map_err(|error| format!("{prog} {op}: {error}"))?;
    Ok((doc, tag, url))
}

fn install(prog: &str, op: &str, doc: &str, tag: &str, url: &str) -> i32 {
    let bar = Progress::new();
    let file_name = url.rsplit('/').next().unwrap_or("toolsboxapp-setup.exe");
    let dest = std::env::temp_dir().join(format!("tbxa-{op}-{}-{file_name}", std::process::id()));
    bar.set(10);
    if let Err(error) = download_file(url, &dest) {
        return fail(&bar, format!("{prog} {op}: {error}"));
    }
    bar.set(50);

    let sums_url = match find_checksums_url(doc) {
        Some(url) => url,
        None => {
            return fail(
                &bar,
                format!("{prog} {op}: no checksums.txt asset in release {tag}; refusing an unverifiable installer."),
            );
        }
    };
    let sums_dest = std::env::temp_dir().join(format!("tbxa-{op}-{}-checksums.txt", std::process::id()));
    if let Err(error) = download_file(&sums_url, &sums_dest) {
        return fail(&bar, format!("{prog} {op}: {error}"));
    }
    bar.set(65);
    let sums = match std::fs::read_to_string(&sums_dest) {
        Ok(sums) => sums,
        Err(error) => {
            return fail(&bar, format!("{prog} {op}: cannot read checksums.txt: {error}"));
        }
    };
    let _ = std::fs::remove_file(&sums_dest);
    let actual = match sha256_of(&dest) {
        Ok(actual) => actual,
        Err(error) => return fail(&bar, format!("{prog} {op}: {error}")),
    };
    if let Err(error) = check_sha256(&sums, file_name, &actual) {
        return fail(&bar, format!("{prog} {op}: {error}"));
    }
    bar.set(85);

    if let Err(error) = run_after_exit(&dest, SILENT_FLAG) {
        return fail(&bar, format!("{prog} {op}: {error}"));
    }
    bar.set(100);

    bar.finish();
    println!("{TEXT_INDENT}ToolsBoXApp {op} started, the installer will finish in a few seconds.");
    0
}

pub fn version(prog: &str) -> i32 {
    match installed_version() {
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

pub fn update(prog: &str) -> i32 {
    let current = match installed_version() {
        Ok(version) => version,
        Err(error) => {
            eprintln!("{prog} update: {error}");
            return 1;
        }
    };

    let (doc, tag, url) = match fetch_latest(prog, "update") {
        Ok(triple) => triple,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    if let Some(latest) = release_version(&url) {
        if !is_newer(&latest, &current) {
            print_banner();
            println!("{TEXT_INDENT}You are already on the latest version of TBXA.");
            return 0;
        }
    }

    print_banner();
    install(prog, "update", &doc, &tag, &url)
}

pub fn reinstall(prog: &str) -> i32 {
    if let Err(error) = installed_version() {
        eprintln!("{prog} reinstall: {error}");
        return 1;
    }

    let (doc, tag, url) = match fetch_latest(prog, "reinstall") {
        Ok(triple) => triple,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    print_banner();
    install(prog, "reinstall", &doc, &tag, &url)
}

pub fn uninstall(prog: &str) -> i32 {
    let command = match uninstaller() {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{prog} uninstall: {error}");
            return 1;
        }
    };

    print_banner();
    let bar = Progress::new();
    bar.set(50);
    if let Err(error) = run_after_exit(Path::new(&command), SILENT_FLAG) {
        return fail(&bar, format!("{prog} uninstall: {error}"));
    }
    bar.set(100);

    bar.finish();
    println!("{TEXT_INDENT}ToolsBoXApp uninstall started, it will finish in a few seconds.");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_registry_values() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\toolsboxapp\r\n    DisplayVersion    REG_SZ    2026.11.14\r\n\r\n";
        assert_eq!(parse_reg_value(output, "DisplayVersion").as_deref(), Some("2026.11.14"));
        assert_eq!(parse_reg_value(output, "UninstallString"), None);
        assert_eq!(parse_reg_value(output, "DisplayVers"), None);
    }

    #[test]
    fn keeps_spaces_in_registry_paths() {
        let output = "    UninstallString    REG_EXPAND_SZ    \"C:\\Users\\Jo Doe\\AppData\\Local\\toolsboxapp\\uninstall.exe\"\n";
        assert_eq!(
            parse_reg_value(output, "UninstallString").as_deref(),
            Some("\"C:\\Users\\Jo Doe\\AppData\\Local\\toolsboxapp\\uninstall.exe\"")
        );
    }
}
