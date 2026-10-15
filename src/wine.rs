use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::Config;

#[path = "client_pin.rs"]
mod client_pin;

const EULA_URL: &str = "https://uuyc.163.com/contact/20240402/40294_1146065.html";
const SETUP_DLL_OVERRIDES: &str = "wevtapi=b,wtsapi32=b";

struct SetupPrefix(PathBuf);

impl Drop for SetupPrefix {
    fn drop(&mut self) {
        stop_wineserver(&self.0);
    }
}

/// One-time provisioning.  Deliberately quiet: progress on stdout, nothing
/// else.  NetEase binaries are never redistributed — they are fetched from
/// the official feed into a per-user prefix.
pub fn setup(accept_eula: bool, repair: bool) -> Result<()> {
    let mut config = Config::load()?;

    if !config.eula_accepted {
        if !accept_eula {
            println!("{}", t!("setup.eula_prompt", url = EULA_URL));
            anyhow::bail!("{}", t!("setup.eula_required"));
        }
        config.eula_accepted = true;
        config.store()?;
        println!("{}", t!("setup.eula_recorded"));
    }

    let _session_lock = crate::session::acquire_session_lock()?;
    let data = crate::config::data_dir()?;
    let prefix = data.join("wine");
    std::fs::create_dir_all(&prefix).context("creating wine prefix directory")?;

    let cache = data.join("cache");
    std::fs::create_dir_all(&cache).context("creating cache directory")?;
    let _prefix_cleanup = SetupPrefix(prefix.clone());

    let initialized = prefix_initialized(&prefix);
    let client_installed = find_client_dir(&prefix).is_some();
    if !repair && initialized && client_installed {
        if !client_service_registered(&prefix) {
            anyhow::bail!(
                "existing UU client has an invalid GameViewerService registration; run `uur setup --repair`"
            );
        }
        finish_setup(&prefix)?;
        println!("{}", t!("setup.already_provisioned"));
        return Ok(());
    }

    println!("{}", t!("setup.initializing_prefix"));
    let status = Command::new("wineboot")
        .env("WINEPREFIX", &prefix)
        .env("WINEDEBUG", wine_debug())
        .env("WINEDLLOVERRIDES", SETUP_DLL_OVERRIDES)
        .arg(if repair { "--update" } else { "--init" })
        .status()
        .context("running wineboot")?;
    if !status.success() {
        anyhow::bail!("{}", t!("setup.prefix_failed", status = status));
    }
    wait_for_wineserver(&prefix)?;
    if !prefix_initialized(&prefix) {
        anyhow::bail!("{}", t!("setup.prefix_failed", status = status));
    }

    if find_client_dir(&prefix).is_some() {
        if !query_client_service(&prefix)? {
            anyhow::bail!(
                "existing UU client has an invalid GameViewerService registration after repair; repair the existing installation before retrying"
            );
        }
        finish_setup(&prefix)?;
        println!("{}", t!("setup.repaired", path = prefix.display()));
        return Ok(());
    }

    let installer = client_pin::acquire(&cache)?;
    clear_canonical_bin_symlink(&prefix);

    println!("{}", t!("setup.installing_client"));
    let status = Command::new("wine")
        .env("WINEPREFIX", &prefix)
        .env("WINEDEBUG", wine_debug())
        .env("WINEDLLOVERRIDES", SETUP_DLL_OVERRIDES)
        .arg(installer.to_string_lossy().as_ref())
        .arg("/S")
        .status()
        .context("launching installer")?;
    if !status.success() {
        anyhow::bail!("{}", t!("setup.install_failed", status = status));
    }

    find_client_dir(&prefix).context(t!("setup.client_incomplete").to_string())?;
    if !query_client_service(&prefix)? {
        anyhow::bail!("{}", t!("setup.service_incomplete"));
    }

    finish_setup(&prefix)?;
    println!("{}", t!("setup.done", path = prefix.display().to_string()));
    Ok(())
}

fn finish_setup(prefix: &Path) -> Result<()> {
    // winemenubuilder generated its entries during the install: adopt the
    // start-menu one for the full session, then stop it from producing raw
    // shortcuts ever again, and sweep desktop copies.
    set_dll_override(prefix, "wevtapi", "native")?;
    set_dll_override(prefix, "winemenubuilder", "")?;
    apply_registry_compat(prefix)?;
    let uur_bin = std::env::current_exe().unwrap_or_default();
    let _ = adopt_shortcut(prefix, &uur_bin);
    purge_wine_shortcuts();

    Ok(())
}

pub fn prefix_initialized(prefix: &Path) -> bool {
    ["system.reg", "user.reg"]
        .iter()
        .all(|name| std::fs::metadata(prefix.join(name)).is_ok_and(|metadata| metadata.len() > 0))
}

pub fn webview2_installed(prefix: &Path) -> bool {
    let application = prefix.join("drive_c/Program Files (x86)/Microsoft/EdgeWebView/Application");
    find_dir_recursive(&application, "msedgewebview2.exe", 3)
        .is_some_and(|path| std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0))
}

/// Read the persisted Wine service registration without starting Wine or
/// showing component installers from `uur doctor`.
pub fn client_service_registered(prefix: &Path) -> bool {
    let Ok(registry) = std::fs::read_to_string(prefix.join("system.reg")) else {
        return false;
    };
    let mut service_section = false;
    for line in registry.lines() {
        if let Some(section) = line.strip_prefix('[').and_then(|line| line.split_once(']')) {
            service_section = section
                .0
                .to_ascii_lowercase()
                .ends_with(r"\\services\\gameviewerservice");
        } else if service_section && line.starts_with("\"ImagePath\"=") {
            return persisted_image_path(line)
                .is_some_and(|command| command_executable_is(&command, "GameViewerService.exe"));
        }
    }
    false
}

fn query_client_service(prefix: &Path) -> Result<bool> {
    let output = Command::new("wine")
        .env("WINEPREFIX", prefix)
        .env("WINEDEBUG", wine_debug())
        .env("WINEDLLOVERRIDES", SETUP_DLL_OVERRIDES)
        .args([
            "reg",
            "query",
            r"HKLM\System\CurrentControlSet\Services\GameViewerService",
            "/v",
            "ImagePath",
        ])
        .stdin(Stdio::null())
        .output()
        .context("checking the UU Windows service registration")?;
    Ok(output.status.success()
        && live_image_path(&String::from_utf8_lossy(&output.stdout))
            .is_some_and(|command| command_executable_is(command, "GameViewerService.exe")))
}

fn persisted_image_path(line: &str) -> Option<String> {
    let encoded = line.strip_prefix("\"ImagePath\"=")?.trim();
    let encoded = encoded.strip_prefix("str(2):").unwrap_or(encoded).trim();
    let encoded = encoded.strip_prefix('"')?.strip_suffix('"')?;
    let mut decoded = String::with_capacity(encoded.len());
    let mut characters = encoded.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some('\\') => decoded.push('\\'),
                Some('"') => decoded.push('"'),
                Some(escaped) => {
                    decoded.push('\\');
                    decoded.push(escaped);
                }
                None => return None,
            }
        } else {
            decoded.push(character);
        }
    }
    Some(decoded)
}

fn live_image_path(output: &str) -> Option<&str> {
    output.lines().find_map(|line| {
        let (_, value) = line
            .split_once("REG_EXPAND_SZ")
            .or_else(|| line.split_once("REG_SZ"))?;
        line.contains("ImagePath").then_some(value.trim())
    })
}

fn command_executable_is(command: &str, expected: &str) -> bool {
    let command = command.trim_start();
    let executable = if let Some(quoted) = command.strip_prefix('"') {
        let Some((executable, _)) = quoted.split_once('"') else {
            return false;
        };
        executable
    } else if let Some(end) = unquoted_exe_end(command) {
        &command[..end]
    } else {
        let Some(executable) = command.split_ascii_whitespace().next() else {
            return false;
        };
        executable
    };
    executable
        .rsplit(['\\', '/'])
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn unquoted_exe_end(command: &str) -> Option<usize> {
    let lower = command.to_ascii_lowercase();
    lower.match_indices(".exe").find_map(|(start, matched)| {
        let end = start + matched.len();
        let tail = &command[end..];
        (tail.is_empty()
            || tail
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_whitespace()))
        .then_some(end)
    })
}

/// Registry compatibility profile, ported from the field-tested reference
/// implementation:
///
/// - msedgewebview2.exe runs under Wine only with a win8 app-compat
///   override — without it the WebView2 process never spawns;
/// - GameViewerServer.exe sees Windows 10 because current UU terminal builds
///   gate their ConPTY transport on the OS version. Native input and display
///   adapters handle the hardware capabilities separately;
/// - the UU client and service prefer their bundled msvcp140 runtime. Wine 10's
///   builtin implementation can stall UU 4.41 before the WebView starts;
/// - the service must auto-start;
/// - X11 focus/XIM tuning and CJK font substitution keep the controller
///   usable.
type RegistryEntry = (String, Vec<(&'static str, &'static str, &'static str)>);

fn registry_compat_entries() -> Vec<RegistryEntry> {
    vec![
        (
            r#"Software\Wine\AppDefaults\msedgewebview2.exe"#.to_string(),
            vec![("Version", "REG_SZ", "win8")],
        ),
        (
            r#"Software\Wine\AppDefaults\GameViewerServer.exe"#.to_string(),
            vec![("Version", "REG_SZ", "win10")],
        ),
        (
            r#"Software\Wine\AppDefaults\gameviewer.exe\DllOverrides"#.to_string(),
            vec![
                ("msvcp140", "REG_SZ", "native,builtin"),
                ("msvcp140_1", "REG_SZ", "native,builtin"),
            ],
        ),
        (
            r#"Software\Wine\AppDefaults\GameViewerServer.exe\DllOverrides"#.to_string(),
            vec![
                ("msvcp140", "REG_SZ", "native,builtin"),
                ("msvcp140_1", "REG_SZ", "native,builtin"),
            ],
        ),
        (
            r#"Software\Wine\AppDefaults\GameViewerService.exe\DllOverrides"#.to_string(),
            vec![
                ("msvcp140", "REG_SZ", "native,builtin"),
                ("msvcp140_1", "REG_SZ", "native,builtin"),
            ],
        ),
        (
            r#"Software\Wine\AppDefaults\GameViewerLauncher.exe\DllOverrides"#.to_string(),
            vec![
                ("msvcp140", "REG_SZ", "native,builtin"),
                ("msvcp140_1", "REG_SZ", "native,builtin"),
            ],
        ),
        (
            r#"Software\Wine\AppDefaults\GameViewerHealthd.exe\DllOverrides"#.to_string(),
            vec![
                ("msvcp140", "REG_SZ", "native,builtin"),
                ("msvcp140_1", "REG_SZ", "native,builtin"),
            ],
        ),
        (
            r"System\CurrentControlSet\Services\GameViewerService".to_string(),
            vec![("Start", "REG_DWORD", "2")],
        ),
        (
            r"Software\Wine\X11 Driver".to_string(),
            vec![("UseTakeFocus", "REG_SZ", "N")],
        ),
        (
            r"Software\Wine\AppDefaults\gameviewer.exe\X11 Driver".to_string(),
            vec![("UseXIM", "REG_SZ", "N"), ("UseTakeFocus", "REG_SZ", "Y")],
        ),
        (
            r"Software\Microsoft\Windows NT\CurrentVersion\FontSubstitutes".to_string(),
            vec![
                ("MS Shell Dlg", "REG_SZ", "Noto Sans CJK SC"),
                ("MS Shell Dlg 2", "REG_SZ", "Noto Sans CJK SC"),
            ],
        ),
    ]
}

pub fn apply_registry_compat(prefix: &Path) -> Result<()> {
    for (key, values) in registry_compat_entries() {
        let full = if key.starts_with("System") {
            format!("HKLM\\{key}")
        } else {
            format!("HKCU\\{key}")
        };
        for (value, reg_type, data) in &values {
            let status = Command::new("wine")
                .env("WINEPREFIX", prefix)
                .env("WINEDEBUG", wine_debug())
                .args([
                    "reg", "add", &full, "/v", value, "/t", reg_type, "/d", data, "/f",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .with_context(|| format!("reg add {full} /v {value}"))?;
            if !status.success() {
                anyhow::bail!("Wine rejected registry compatibility setting {full} /v {value}");
            }
        }
    }
    Ok(())
}

pub fn set_dll_override(prefix: &Path, name: &str, mode: &str) -> Result<()> {
    let status = Command::new("wine")
        .env("WINEPREFIX", prefix)
        .env("WINEDEBUG", wine_debug())
        .args([
            "reg",
            "add",
            r"HKCU\Software\Wine\DllOverrides",
            "/v",
            name,
            "/t",
            "REG_SZ",
            "/d",
            mode,
            "/f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("registering {name} dll override"))?;
    if !status.success() {
        anyhow::bail!("Wine rejected the {name} DLL override");
    }
    Ok(())
}

pub fn set_log_pixels(prefix: &Path, dpi: u32) -> Result<()> {
    let status = log_pixels_command(prefix, dpi)
        .status()
        .context("setting the Wine system DPI")?;
    if !status.success() {
        anyhow::bail!("Wine rejected the system DPI value {dpi}");
    }
    Ok(())
}

fn log_pixels_command(prefix: &Path, dpi: u32) -> Command {
    let mut command = Command::new("wine");
    command
        .env("WINEPREFIX", prefix)
        .env("WINEDEBUG", wine_debug())
        .args([
            "reg",
            "add",
            r"HKCU\Control Panel\Desktop",
            "/v",
            "LogPixels",
            "/t",
            "REG_DWORD",
            "/d",
            &dpi.to_string(),
            "/f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn wine_debug() -> String {
    std::env::var("WINEDEBUG").unwrap_or_else(|_| "-all".to_string())
}

fn stop_wineserver(prefix: &Path) {
    let _ = Command::new("wineserver")
        .env("WINEPREFIX", prefix)
        .arg("-k")
        .status();
}

fn wait_for_wineserver(prefix: &Path) -> Result<()> {
    let status = wineserver_wait_command(prefix)
        .status()
        .context("waiting for wineboot processes")?;
    if status.code() == Some(124) {
        anyhow::bail!("timed out after 60 seconds waiting for Wine prefix initialization");
    }
    if !status.success() {
        anyhow::bail!("wineserver failed while waiting for Wine prefix initialization");
    }
    Ok(())
}

fn wineserver_wait_command(prefix: &Path) -> Command {
    let mut command = Command::new("timeout");
    command
        .env("WINEPREFIX", prefix)
        .args(["--kill-after=5s", "60s", "wineserver", "-w"]);
    command
}

/// Locate the installed client directory regardless of how NetEase spells
/// the vendor folder ("Netease", "NetEase", ...).  Case matters on Linux.
pub fn find_client_dir(prefix: &Path) -> Option<PathBuf> {
    let program_files = prefix.join("drive_c").join("Program Files");
    let vendor = find_child_dir(&program_files, "netease")?;
    let canonical = find_child_dir(&vendor, "gameviewer").unwrap_or(vendor);
    repair_canonical_bin_link(&canonical);
    if client_root_is_complete(&canonical) {
        return Some(canonical);
    }
    sorted_child_dirs(&canonical)
        .into_iter()
        .find(|candidate| client_root_is_complete(candidate))
}

fn clear_canonical_bin_symlink(prefix: &Path) {
    let program_files = prefix.join("drive_c").join("Program Files");
    let Some(vendor) = find_child_dir(&program_files, "netease") else {
        return;
    };
    let canonical = find_child_dir(&vendor, "gameviewer").unwrap_or(vendor);
    let bin = canonical.join("bin");
    if std::fs::symlink_metadata(&bin).is_ok_and(|meta| meta.file_type().is_symlink()) {
        let _ = std::fs::remove_file(&bin);
    }
}

fn repair_canonical_bin_link(canonical: &Path) {
    if !find_child_file(canonical, "GameViewer.exe").is_some_and(|path| nonempty_file(&path)) {
        return;
    }
    let Some(best_version) = latest_version_payload_dir(canonical) else {
        return;
    };
    let Some(version_name) = best_version.file_name() else {
        return;
    };
    let bin = canonical.join("bin");
    let is_symlink =
        std::fs::symlink_metadata(&bin).is_ok_and(|meta| meta.file_type().is_symlink());
    let metadata_missing = std::fs::symlink_metadata(&bin).is_err();
    if !is_symlink && !metadata_missing {
        return;
    }
    let already_current = is_symlink
        && std::fs::read_link(&bin).is_ok_and(|target| target == Path::new(version_name))
        && find_child_file(&bin, "GameViewerServer.exe").is_some_and(|path| nonempty_file(&path));
    if already_current {
        return;
    }
    if is_symlink {
        let _ = std::fs::remove_file(&bin);
    }
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(version_name, &bin);
    }
}

fn latest_version_payload_dir(canonical: &Path) -> Option<PathBuf> {
    let mut candidates = sorted_child_dirs(canonical)
        .into_iter()
        .filter(|dir| {
            !dir.file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
                && find_child_file(dir, "GameViewer.exe").is_some_and(|path| nonempty_file(&path))
                && find_child_file(dir, "GameViewerServer.exe")
                    .is_some_and(|path| nonempty_file(&path))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        version_sort_key(left)
            .cmp(&version_sort_key(right))
            .then_with(|| lowercase_file_name(left).cmp(&lowercase_file_name(right)))
    });
    candidates.pop()
}

fn version_sort_key(path: &Path) -> Vec<u32> {
    let name = path
        .file_name()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy();
    let mut parts = Vec::new();
    for segment in name.split('.') {
        let Ok(number) = segment.parse::<u32>() else {
            return Vec::new();
        };
        parts.push(number);
    }
    parts
}

fn client_root_is_complete(root: &Path) -> bool {
    find_child_file(root, "GameViewer.exe").is_some_and(|path| nonempty_file(&path))
        && (find_child_dir(root, "bin")
            .and_then(|bin| find_child_file(&bin, "GameViewerServer.exe"))
            .is_some_and(|path| nonempty_file(&path))
            || sorted_child_dirs(root).into_iter().any(|version| {
                !version
                    .file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
                    && (find_child_file(&version, "GameViewerServer.exe")
                        .is_some_and(|path| nonempty_file(&path))
                        || find_child_dir(&version, "bin")
                            .and_then(|bin| find_child_file(&bin, "GameViewerServer.exe"))
                            .is_some_and(|path| nonempty_file(&path)))
            }))
}

fn nonempty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn find_child_dir(parent: &Path, name: &str) -> Option<PathBuf> {
    sorted_child_dirs(parent).into_iter().find(|path| {
        path.file_name()
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
    })
}

fn find_child_file(parent: &Path, name: &str) -> Option<PathBuf> {
    sorted_children(parent).into_iter().find(|path| {
        path.is_file()
            && path
                .file_name()
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(name))
    })
}

fn sorted_child_dirs(parent: &Path) -> Vec<PathBuf> {
    sorted_children(parent)
        .into_iter()
        .filter(|path| path.is_dir())
        .collect()
}

fn sorted_children(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        lowercase_file_name(left)
            .cmp(&lowercase_file_name(right))
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });
    paths
}

fn lowercase_file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy()
        .to_ascii_lowercase()
}

fn find_dir_recursive(start: &Path, needle: &str, depth: u8) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(start).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(needle))
        {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_dir_recursive(&path, needle, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Preserve an existing Wine-generated launcher as a hidden compatibility
/// alias. Previously pinned desktop icons keep working, but the packaged
/// `uur.desktop` remains the one visible menu entry.
pub fn adopt_shortcut(prefix: &Path, uur_bin: &Path) -> Result<()> {
    use std::fs;

    let mut name = None;
    let users = prefix.join("drive_c/users");
    if let Ok(entries) = fs::read_dir(&users) {
        'outer: for user in entries.flatten() {
            let programs = user
                .path()
                .join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs");
            let Ok(items) = fs::read_dir(&programs) else {
                continue;
            };
            for item in items.flatten() {
                let file_name = item.file_name().to_string_lossy().into_owned();
                if file_name.ends_with(".lnk") && shortcut_targets_gameviewer(&item.path()) {
                    name = Some(file_name.trim_end_matches(".lnk").to_string());
                    break 'outer;
                }
            }
        }
    }
    let Some(name) = name else {
        return Ok(());
    };

    let mut icon_name = String::new();
    let data_home = crate::config::data_dir()?
        .parent()
        .context("no data home")?
        .to_path_buf();
    let icons = data_home.join("icons");
    let mut best = 0u32;
    if let Ok(sizes) = fs::read_dir(icons.join("hicolor")) {
        for size in sizes.flatten() {
            let size_name = size.file_name().to_string_lossy().into_owned();
            let Ok(width) = size_name.split('x').next().unwrap_or("").parse::<u32>() else {
                continue;
            };
            let Ok(apps) = fs::read_dir(size.path().join("apps")) else {
                continue;
            };
            for app in apps.flatten() {
                let file_name = app.file_name().to_string_lossy().into_owned();
                if file_name.contains("GameViewer") && width > best {
                    best = width;
                    icon_name = file_name
                        .trim_end_matches(".png")
                        .trim_end_matches(".0")
                        .to_string();
                }
            }
        }
    }

    let applications = data_home.join("applications");
    std::fs::create_dir_all(&applications)?;
    let entry = applications.join(format!("{name}.desktop"));
    let exec = uur_bin.display();
    let icon_line = if icon_name.is_empty() {
        String::new()
    } else {
        format!("Icon={icon_name}\n")
    };
    std::fs::write(
        &entry,
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={name}\n\
             Comment=UU Remote for Linux (uur)\n\
             Exec={exec} run\n\
             TryExec={exec}\n\
             {icon_line}Terminal=false\n\
             NoDisplay=true\n\
             X-UUR-Managed=true\n\
             X-UUR-Prefix={}\n\
             Categories=Network;RemoteAccess;\n",
            prefix.display()
        ),
    )
    .with_context(|| format!("writing {}", entry.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&entry, fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

fn shortcut_targets_gameviewer(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let searchable: String = bytes
        .into_iter()
        .filter(|byte| *byte != 0)
        .map(|byte| byte as char)
        .collect::<String>()
        .to_ascii_lowercase();
    searchable.contains("gameviewer")
}

/// Remove Wine-generated desktop copies of the client.  The start-menu
/// entry is ours and correct; only Desktop copies are swept.
fn purge_wine_shortcuts() {
    let home = match std::env::var("HOME") {
        Ok(home) => PathBuf::from(home),
        Err(_) => return,
    };
    let matches = |name: &str| {
        let lower = name.to_lowercase();
        lower.contains("uuremote") || lower.contains("gameviewer") || lower.contains("uu remote")
    };
    let dir = home.join("Desktop");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches(&name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::client_service_registered;
    use super::command_executable_is;
    use super::find_client_dir;
    use super::live_image_path;
    use super::log_pixels_command;
    use super::registry_compat_entries;
    use super::wineserver_wait_command;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct FixtureDir(PathBuf);

    impl FixtureDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "uur-wine-{name}-{}-{}",
                std::process::id(),
                FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            if path.exists() {
                fs::remove_dir_all(&path).expect("remove stale test fixture");
            }
            fs::create_dir_all(&path).expect("create test fixture");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for FixtureDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove test fixture");
        }
    }

    fn write_client(root: &Path, service_name: &str) {
        fs::create_dir_all(root.join("bin")).expect("create client bin directory");
        fs::write(root.join("GameViewer.exe"), b"client").expect("write client fixture");
        fs::write(root.join("bin").join(service_name), b"service").expect("write service fixture");
    }

    fn write_service_registry(prefix: &Path, image_path: &str) {
        fs::write(
            prefix.join("system.reg"),
            format!(
                "[System\\\\ControlSet001\\\\Services\\\\GameViewerService]\n\"ImagePath\"=str(2):\"{image_path}\"\n"
            ),
        )
        .expect("write service registry fixture");
    }

    #[test]
    fn bundled_cpp_runtime_is_preferred_for_both_uu_processes() {
        let entries = registry_compat_entries();
        for executable in ["gameviewer.exe", "GameViewerServer.exe"] {
            let key = format!(r"Software\Wine\AppDefaults\{executable}\DllOverrides");
            let values = entries
                .iter()
                .find_map(|(entry, values)| (entry == &key).then_some(values))
                .unwrap_or_else(|| panic!("missing DLL overrides for {executable}"));
            assert!(values.contains(&("msvcp140", "REG_SZ", "native,builtin")));
        }
    }

    #[test]
    fn service_registration_accepts_actual_service_executable() {
        let fixture = FixtureDir::new("service-valid");
        write_service_registry(
            fixture.path(),
            r#"\"C:\\Program Files\\Netease\\GameViewer\\GameViewerService.exe\" --service"#,
        );

        assert!(client_service_registered(fixture.path()));
    }

    #[test]
    fn persisted_registry_parser_accepts_direct_reg_sz_image_path() {
        let fixture = FixtureDir::new("service-direct-reg-sz");
        fs::write(
            fixture.path().join("system.reg"),
            r#"[System\\ControlSet001\\Services\\GameViewerService]
"ImagePath"="C:\\Program Files\\Netease\\GameViewer\\GameViewerService.exe --service"
"#,
        )
        .expect("write direct REG_SZ registry fixture");

        assert!(client_service_registered(fixture.path()));
    }

    #[test]
    fn service_registration_rejects_server_executable() {
        let fixture = FixtureDir::new("service-server");
        write_service_registry(
            fixture.path(),
            r"C:\Program Files\NetEase\GameViewer\bin\GameViewerServer.exe --service",
        );

        assert!(!client_service_registered(fixture.path()));
    }

    #[test]
    fn service_registration_preserves_mixed_case_matching() {
        let fixture = FixtureDir::new("service-mixed-case");
        write_service_registry(
            fixture.path(),
            r#"\"C:\\Program Files\\Netease\\GameViewer\\gAmEvIeWeRsErViCe.ExE\" --service"#,
        );

        assert!(client_service_registered(fixture.path()));
    }

    #[test]
    fn service_registration_rejects_executable_substring_spoof() {
        let fixture = FixtureDir::new("service-spoof");
        write_service_registry(
            fixture.path(),
            r"C:\Program Files\NetEase\GameViewer\bin\NotGameViewerService.exe.bak",
        );

        assert!(!client_service_registered(fixture.path()));
    }

    #[test]
    fn service_registration_rejects_service_name_as_unrelated_argument() {
        let fixture = FixtureDir::new("service-argument-spoof");
        write_service_registry(
            fixture.path(),
            r#"\"C:\\Windows\\System32\\notepad.exe\" --note GameViewerService.exe"#,
        );

        assert!(!client_service_registered(fixture.path()));
    }

    #[test]
    fn live_registry_parser_checks_image_path_executable() {
        let output = concat!(
            "HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Services\\GameViewerService\r\n",
            "    ImagePath    REG_EXPAND_SZ    \"C:\\Program Files\\Netease\\GameViewer\\GameViewerService.exe\" --service\r\n",
        );

        let image_path = live_image_path(output).expect("parse live ImagePath column");

        assert!(command_executable_is(image_path, "GameViewerService.exe"));
    }

    #[test]
    fn live_registry_parser_rejects_service_name_as_argument() {
        let output = concat!(
            "HKEY_LOCAL_MACHINE\\System\\CurrentControlSet\\Services\\GameViewerService\r\n",
            "    ImagePath    REG_SZ    notepad.exe --note GameViewerService.exe\r\n",
        );

        let image_path = live_image_path(output).expect("parse live ImagePath column");

        assert!(!command_executable_is(image_path, "GameViewerService.exe"));
    }

    #[test]
    fn wineserver_wait_is_bounded_to_sixty_seconds() {
        let prefix = std::env::temp_dir().join("uur-wine-wait-command");

        let command = wineserver_wait_command(&prefix);
        let args = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(args, ["--kill-after=5s", "60s", "wineserver", "-w"]);
        assert_eq!(
            command.get_envs().find_map(|(name, value)| {
                (name == "WINEPREFIX").then(|| value.map(PathBuf::from))
            }),
            Some(Some(prefix))
        );
    }

    #[test]
    fn client_discovery_prefers_canonical_root_over_version_subdirectory() {
        let fixture = FixtureDir::new("client-canonical");
        let vendor = fixture
            .path()
            .join("drive_c")
            .join("Program Files")
            .join("NetEase");
        let canonical = vendor.join("GameViewer");
        write_client(&canonical, "GameViewerServer.exe");
        write_client(&canonical.join("4.42.1.2835"), "GameViewerServer.exe");

        assert_eq!(find_client_dir(fixture.path()), Some(canonical));
    }

    #[test]
    fn client_discovery_returns_canonical_root_for_versioned_server_layout() {
        let fixture = FixtureDir::new("client-canonical-versioned-server");
        let vendor = fixture
            .path()
            .join("drive_c")
            .join("Program Files")
            .join("NetEase");
        let canonical = vendor.join("GameViewer");
        fs::create_dir_all(canonical.join("4.41.1.2335").join("bin"))
            .expect("create versioned client bin directory");
        fs::write(canonical.join("GameViewer.exe"), b"client")
            .expect("write canonical client fixture");
        fs::write(
            canonical
                .join("4.41.1.2335")
                .join("bin")
                .join("GameViewerServer.exe"),
            b"service",
        )
        .expect("write versioned service fixture");

        assert_eq!(find_client_dir(fixture.path()), Some(canonical));
    }

    #[cfg(unix)]
    #[test]
    fn client_discovery_repairs_dangling_bin_symlink_to_latest_version() {
        let fixture = FixtureDir::new("client-repair-bin-symlink");
        let canonical = fixture
            .path()
            .join("drive_c")
            .join("Program Files")
            .join("Netease")
            .join("GameViewer");
        let backup = canonical.join("1.0.0.0");
        let latest = canonical.join("4.42.1.2835");
        fs::create_dir_all(&backup).expect("create backup version directory");
        fs::create_dir_all(&latest).expect("create latest version directory");
        fs::write(canonical.join("GameViewer.exe"), b"launcher").expect("write launcher");
        fs::write(backup.join("GameViewer.exe"), b"old-ui").expect("write old ui");
        fs::write(backup.join("GameViewerServer.exe"), b"old-srv").expect("write old server");
        fs::write(latest.join("GameViewer.exe"), b"new-ui").expect("write new ui");
        fs::write(latest.join("GameViewerServer.exe"), b"new-srv").expect("write new server");
        std::os::unix::fs::symlink("4.41.1.2335", canonical.join("bin"))
            .expect("create dangling bin symlink");

        assert_eq!(find_client_dir(fixture.path()), Some(canonical.clone()));
        assert_eq!(
            fs::read(canonical.join("bin").join("GameViewerServer.exe")).expect("read linked bin"),
            b"new-srv"
        );
    }

    #[test]
    fn client_discovery_accepts_version_before_mixed_case_bin() {
        let fixture = FixtureDir::new("client-versioned");
        let versioned = fixture
            .path()
            .join("drive_c")
            .join("Program Files")
            .join("nEtEaSe")
            .join("GameViewer")
            .join("4.42.1.2835");
        fs::create_dir_all(versioned.join("BiN")).expect("create mixed-case bin directory");
        fs::write(versioned.join("gAmEvIeWeR.ExE"), b"client")
            .expect("write mixed-case client fixture");
        fs::write(
            versioned.join("BiN").join("gAmEvIeWeRsErVeR.ExE"),
            b"service",
        )
        .expect("write mixed-case service fixture");

        assert_eq!(find_client_dir(fixture.path()), Some(versioned));
    }

    #[test]
    fn system_dpi_command_writes_winecfg_logpixels_value() {
        let prefix = std::env::temp_dir().join("uur-wine-dpi-command");
        let command = log_pixels_command(&prefix, 192);
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "reg",
                "add",
                r"HKCU\Control Panel\Desktop",
                "/v",
                "LogPixels",
                "/t",
                "REG_DWORD",
                "/d",
                "192",
                "/f",
            ]
        );
    }
}
