//! Keeping Shum up to date. The background service checks GitHub once a day
//! and updates the CLI the way it was installed: Homebrew on macOS, or the
//! install script on macOS and Windows; the services then restart on the new
//! version. `shum update` does the same at once, and `shum update --auto off`
//! turns the background updates off. Development builds never update.
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const RELEASES: &str = "https://api.github.com/repos/hiTechTeam/Shum-CLI/releases";
const INSTALL_SH: &str = "https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh";
const INSTALL_PS1: &str = "https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.ps1";
const DAY: u64 = 24 * 60 * 60;

pub const VERSION: &str = env!("SHUM_VERSION");

/// How this copy of Shum was installed, which decides how it updates.
#[derive(Clone, Debug, PartialEq)]
pub enum Install {
    Homebrew { brew: PathBuf },
    Script,
    Development,
}

pub fn install() -> Install {
    let exe = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .unwrap_or_default();
    let path = exe.to_string_lossy().replace('\\', "/");
    if path.contains("/Cellar/shum/") {
        let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.exists());
        if let Some(brew) = brew {
            return Install::Homebrew { brew };
        }
    }
    if path.contains("/.local/share/shum/Shum.app/") || path.contains("/Programs/Shum/") {
        return Install::Script;
    }
    Install::Development
}

fn numbers(version: &str) -> Vec<u64> {
    version
        .trim_start_matches("windows-v")
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

/// Whether `latest` is a newer version than `current`.
pub fn newer(latest: &str, current: &str) -> bool {
    numbers(latest) > numbers(current)
}

fn fetch(url: &str) -> Result<Value> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "30",
            "-H",
            "Accept: application/vnd.github+json",
            url,
        ])
        .stdin(Stdio::null())
        .output()
        .context("curl")?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// The newest released version for this system: the latest release on
/// macOS, the newest `windows-v*` preview on Windows.
pub fn latest() -> Result<String> {
    if cfg!(windows) {
        let releases = fetch(&format!("{RELEASES}?per_page=30"))?;
        releases
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["draft"] != true)
            .filter_map(|r| r["tag_name"].as_str())
            .find_map(|tag| tag.strip_prefix("windows-v").map(str::to_owned))
            .context("no Windows release")
    } else {
        let release = fetch(&format!("{RELEASES}/latest"))?;
        release["tag_name"]
            .as_str()
            .map(|tag| tag.trim_start_matches('v').to_owned())
            .context("no release")
    }
}

/// The shell command that updates this installation and restarts the
/// services on the new version.
fn command(install: &Install) -> Option<Command> {
    match install {
        Install::Homebrew { brew } => {
            let brew = brew.display().to_string().replace('\'', "");
            let mut command = Command::new("/bin/sh");
            command.arg("-c").arg(format!(
                "'{brew}' update --quiet && '{brew}' upgrade hitechteam/shum/shum && \"$('{brew}' --prefix)/bin/shum\" daemon --refresh"
            ));
            Some(command)
        }
        Install::Script if cfg!(windows) => {
            let mut command = Command::new("powershell");
            command.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &format!("irm {INSTALL_PS1} | iex"),
            ]);
            Some(command)
        }
        Install::Script => {
            let mut command = Command::new("/bin/sh");
            command
                .arg("-c")
                .arg(format!("curl -fsSL {INSTALL_SH} | sh"));
            Some(command)
        }
        Install::Development => None,
    }
}

/// Updates now and waits, for `shum update`. Returns the version installed,
/// or `None` when this one is already the latest.
pub fn update_now() -> Result<Option<String>> {
    let latest = latest()?;
    if !newer(&latest, VERSION) {
        return Ok(None);
    }
    let Some(mut command) = command(&install()) else {
        bail!(
            "{}",
            crate::i18n::t("Эту установку Shum обновить автоматически нельзя")
        );
    };
    let status = command.status()?;
    if !status.success() {
        bail!("{status}");
    }
    Ok(Some(latest))
}

fn settings_path(root: &Path) -> PathBuf {
    root.join("updates.json")
}

fn settings(root: &Path) -> Value {
    std::fs::read(settings_path(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| json!({}))
}

fn save(root: &Path, value: &Value) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    std::io::Write::write_all(&mut file, &serde_json::to_vec(value)?)?;
    file.persist(settings_path(root))?;
    Ok(())
}

/// Whether the background service updates Shum (on unless turned off).
pub fn auto(root: &Path) -> bool {
    settings(root)["auto"] != false
}

pub fn set_auto(root: &Path, on: bool) -> Result<()> {
    let mut value = settings(root);
    value["auto"] = json!(on);
    save(root, &value)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// One background check: at most once a day for all profiles of this data
/// folder. The update runs detached, because restarting the services also
/// stops the one that started it.
fn check(root: &Path) -> Result<()> {
    if !auto(root) {
        return Ok(());
    }
    let install = install();
    if install == Install::Development {
        return Ok(());
    }
    let lock = std::fs::File::create(root.join("updates.lock"))?;
    if fs2::FileExt::try_lock_exclusive(&lock).is_err() {
        return Ok(());
    }
    let mut value = settings(root);
    if now().saturating_sub(value["checked"].as_u64().unwrap_or(0)) < DAY {
        return Ok(());
    }
    let latest = latest()?;
    value["checked"] = json!(now());
    save(root, &value)?;
    if !newer(&latest, VERSION) {
        return Ok(());
    }
    eprintln!("Shum update: {VERSION} -> {latest}");
    let Some(mut command) = command(&install) else {
        return Ok(());
    };
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("update.log"))?;
    command
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Detached, in its own group, without a console window.
        command.creation_flags(0x00000008 | 0x00000200 | 0x08000000);
    }
    command.spawn()?;
    Ok(())
}

/// Starts the daily background check in the service.
pub fn watch(root: PathBuf) {
    tokio::spawn(async move {
        // Let the service settle first; a fresh start may come from an update.
        tokio::time::sleep(Duration::from_secs(5 * 60)).await;
        loop {
            let folder = root.clone();
            if let Ok(Err(error)) = tokio::task::spawn_blocking(move || check(&folder)).await {
                eprintln!("Shum update check failed: {error}");
            }
            tokio::time::sleep(Duration::from_secs(60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.1.15", "0.1.14"));
        assert!(newer("v0.1.10", "0.1.9"));
        assert!(newer("windows-v0.1.10", "0.1.9"));
        assert!(!newer("0.1.14", "0.1.14"));
        assert!(!newer("0.1.13", "0.1.14"));
    }

    #[test]
    fn background_updates_are_on_until_turned_off() {
        let dir = tempfile::tempdir().unwrap();
        assert!(auto(dir.path()));
        set_auto(dir.path(), false).unwrap();
        assert!(!auto(dir.path()));
        set_auto(dir.path(), true).unwrap();
        assert!(auto(dir.path()));
    }
}
