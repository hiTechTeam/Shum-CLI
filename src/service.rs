use crate::i18n::t;
use anyhow::{bail, Context, Result};
use std::{path::Path, process::Command};
fn run(command: &mut Command) -> Result<()> {
    let result = command.output()?;
    if !result.status.success() {
        bail!(
            "{}",
            crate::i18n::format(
                "Системная служба: {}",
                &[(
                    "0",
                    crate::terminal::safe(&String::from_utf8_lossy(&result.stderr)).to_string()
                )]
            )
        );
    }
    Ok(())
}
pub fn install(root: &Path, id: &str) -> Result<()> {
    if !profile_id(id) {
        bail!("{}", t("Недействительный ID профиля"));
    }
    let root = root.canonicalize()?;
    let root = root.as_path();
    #[cfg(not(target_os = "macos"))]
    let exe = std::env::current_exe()?;
    #[cfg(target_os = "macos")]
    {
        let exe = crate::macos::prepare(root)?.join("Contents/MacOS/shum");
        fn xml(value: &str) -> String {
            value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        }
        let home = directories::BaseDirs::new()
            .context("Домашний каталог недоступен")?
            .home_dir()
            .to_owned();
        let directory = home.join("Library/LaunchAgents");
        std::fs::create_dir_all(&directory)?;
        let label = format!("org.shum.cli.{id}");
        let file = directory.join(format!("{label}.plist"));
        let args = [
            exe.display().to_string(),
            "--data-dir".into(),
            root.display().to_string(),
            "--profile".into(),
            id.into(),
            "daemon".into(),
            "--run".into(),
        ]
        .iter()
        .map(|s| format!("<string>{}</string>", xml(s)))
        .collect::<String>();
        let log = xml(&root.join(id).join("service.log").display().to_string());
        let plist=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{label}</string><key>AssociatedBundleIdentifiers</key><array><string>org.shum.cli</string></array><key>ProgramArguments</key><array>{args}</array><key>RunAtLoad</key><true/><key>KeepAlive</key><false/><key>StandardOutPath</key><string>{log}</string><key>StandardErrorPath</key><string>{log}</string></dict></plist>");
        std::fs::write(&file, plist)?;
        let uid = Command::new("id").arg("-u").output()?;
        let uid = std::str::from_utf8(&uid.stdout)?.trim();
        let _ = Command::new("launchctl")
            .args(["bootout", &format!("gui/{uid}/{label}")])
            .output();
        run(Command::new("launchctl")
            .args(["bootstrap", &format!("gui/{uid}")])
            .arg(file))?;
    }
    #[cfg(target_os = "linux")]
    {
        fn quote(value: &str) -> String {
            format!(
                "\"{}\"",
                value
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('%', "%%")
                    .replace('$', "$$")
            )
        }
        let config = directories::BaseDirs::new()
            .context(t("Нет каталога конфигурации"))?
            .config_dir()
            .join("systemd/user");
        std::fs::create_dir_all(&config)?;
        let name = format!("shum-{id}.service");
        let unit=format!("[Unit]\nDescription=Shum profile {id}\nAfter=network-online.target\n[Service]\nType=simple\nExecStart={} --data-dir {} --profile {id} daemon --run\nRestart=on-failure\nRestartSec=10\n[Install]\nWantedBy=default.target\n",quote(&exe.display().to_string()),quote(&root.display().to_string()));
        std::fs::write(config.join(&name), unit)?;
        run(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        run(Command::new("systemctl").args(["--user", "enable", "--now", &name]))?;
    }
    #[cfg(target_os = "windows")]
    {
        let arguments = format!(
            "\"{}\" --data-dir \"{}\" --profile {} daemon --run",
            exe.display(),
            root.display(),
            id
        );
        run(Command::new("schtasks").args([
            "/Create",
            "/F",
            "/SC",
            "ONLOGON",
            "/TN",
            &format!("Shum-{id}"),
            "/TR",
            &arguments,
        ]))?;
        run(Command::new("schtasks").args(["/Run", "/TN", &format!("Shum-{id}")]))?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    bail!("{}", t("Автозапуск на этой системе не поддерживается"));
    Ok(())
}

/// Serialize service replacement and uninstall without locking the profile database.
/// The daemon itself never takes this lock.
pub async fn control_lock(root: &Path) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let path = root.join("service-control.lock");
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            bail!("{}", t("Недействительный файл блокировки служб"));
        }
    }
    let file = options.open(path)?;
    for _ in 0..900 {
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => return Ok(file),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await
            }
            Err(error) => return Err(error.into()),
        }
    }
    bail!(
        "{}",
        t("Другая команда управления службами не завершилась за 90 секунд")
    )
}

fn profile_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(target_os = "macos")]
fn agent_directory() -> Result<std::path::PathBuf> {
    Ok(directories::BaseDirs::new()
        .context(t("Домашний каталог недоступен"))?
        .home_dir()
        .join("Library/LaunchAgents"))
}

#[cfg(target_os = "macos")]
struct Agent {
    file: std::path::PathBuf,
    root: std::path::PathBuf,
    id: String,
}

#[cfg(target_os = "macos")]
fn read_agent(file: &Path) -> Result<Agent> {
    let meta = std::fs::symlink_metadata(file)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 1024 * 1024 {
        bail!(
            "{}",
            crate::i18n::format(
                "Недействительный LaunchAgent: {}",
                &[("0", format!("{}", file.display()))]
            )
        );
    }
    let result = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(file)
        .output()?;
    if !result.status.success() {
        bail!(
            "{}",
            crate::i18n::format(
                "Не удалось прочитать LaunchAgent: {}",
                &[("0", format!("{}", file.display()))]
            )
        );
    }
    let value: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    let label = value["Label"]
        .as_str()
        .context(t("LaunchAgent без Label"))?;
    let id = label
        .strip_prefix("org.shum.cli.")
        .filter(|id| profile_id(id))
        .context(t("Недействительный Label Shum"))?;
    if file.file_name().and_then(|n| n.to_str()) != Some(&format!("{label}.plist")) {
        bail!("{}", t("Имя LaunchAgent не соответствует Label"));
    }
    let args: Vec<String> = serde_json::from_value(value["ProgramArguments"].clone())?;
    if args.len() != 7
        || args[1] != "--data-dir"
        || args[3] != "--profile"
        || args[4] != id
        || args[5] != "daemon"
        || args[6] != "--run"
    {
        bail!(
            "{}",
            crate::i18n::format(
                "Неизвестные аргументы LaunchAgent: {}",
                &[("0", format!("{}", file.display()))]
            )
        );
    }
    let root = std::path::PathBuf::from(&args[2]);
    if !root.is_absolute() {
        bail!("{}", t("Каталог данных LaunchAgent должен быть абсолютным"));
    }
    Ok(Agent {
        file: file.into(),
        root: root.canonicalize().unwrap_or(root),
        id: id.into(),
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn is_installed(root: &Path, id: &str) -> Result<bool> {
    let file = agent_directory()?.join(format!("org.shum.cli.{id}.plist"));
    if !file.exists() {
        return Ok(false);
    }
    let agent = read_agent(&file)?;
    if agent.root != root.canonicalize()? {
        bail!(
            "{}",
            t("LaunchAgent этого профиля использует другой каталог данных")
        );
    }
    Ok(true)
}

#[cfg(target_os = "macos")]
fn bootout(id: &str) -> Result<()> {
    let uid = Command::new("/usr/bin/id").arg("-u").output()?;
    let uid = std::str::from_utf8(&uid.stdout)?.trim();
    let target = format!("gui/{uid}/org.shum.cli.{id}");
    if Command::new("/bin/launchctl")
        .args(["print", &target])
        .output()?
        .status
        .success()
    {
        run(Command::new("/bin/launchctl").args(["bootout", &target]))?;
    }
    Ok(())
}

type ServiceRoots = std::collections::BTreeSet<std::path::PathBuf>;
type ServiceProfiles = std::collections::BTreeSet<(std::path::PathBuf, String)>;
#[cfg(not(target_os = "macos"))]
struct Agent;

type ServiceDiscovery = (ServiceRoots, ServiceProfiles, Vec<Agent>);

fn service_profiles(root: &Path, all_registered: bool) -> Result<ServiceDiscovery> {
    let root = root.canonicalize().unwrap_or_else(|_| root.into());
    // Only macOS discovers other registered agents and adds their roots.
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut roots = std::collections::BTreeSet::from([root.clone()]);
    #[cfg(target_os = "macos")]
    let agents = {
        let directory = agent_directory()?;
        let mut agents = Vec::new();
        if directory.exists() {
            for entry in std::fs::read_dir(directory)? {
                let file = entry?.path();
                let Some(name) = file.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !name.starts_with("org.shum.cli.") || !name.ends_with(".plist") {
                    continue;
                }
                let agent = read_agent(&file)?;
                if all_registered || agent.root == root {
                    roots.insert(agent.root.clone());
                    agents.push(agent);
                }
            }
        }
        agents
    };
    #[cfg(not(target_os = "macos"))]
    let _ = all_registered;
    #[cfg(not(target_os = "macos"))]
    let agents = Vec::new();
    let mut profiles = std::collections::BTreeSet::new();
    for root in &roots {
        if !root.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let id = entry.file_name().to_string_lossy().into_owned();
            if profile_id(&id) && entry.file_type()?.is_dir() {
                profiles.insert((root.clone(), id));
            }
        }
    }
    #[cfg(target_os = "macos")]
    for agent in &agents {
        profiles.insert((agent.root.clone(), agent.id.clone()));
    }
    Ok((roots, profiles, agents))
}

/// Refresh only active services after an installer switches the stable bundle.
/// Inactive profiles stay inactive; discovery never opens their keys.
pub async fn refresh(root: &Path, all_registered: bool) -> Result<serde_json::Value> {
    let (_, profiles, _) = service_profiles(root, all_registered)?;
    let mut refreshed = 0;
    for (root, id) in profiles {
        if crate::ipc::request(&root, &id, crate::runtime::Request::Snapshot)
            .await
            .is_ok()
        {
            crate::ipc::ensure(&root, &id).await?;
            refreshed += 1;
        }
    }
    Ok(serde_json::json!({"activeProfilesRefreshed": refreshed}))
}

/// With an explicit data directory, only uninstall that root. Otherwise include
/// all Shum roots registered in the user's LaunchAgents. Never open a key vault.
pub async fn uninstall(root: &Path, all_registered: bool) -> Result<serde_json::Value> {
    uninstall_inner(root, all_registered, false).await
}

/// Explicit, confirmed deletion of profile vaults and history. Package files are
/// owned by the package manager and must be uninstalled separately.
pub async fn purge(root: &Path, all_registered: bool) -> Result<serde_json::Value> {
    uninstall_inner(root, all_registered, true).await
}

async fn uninstall_inner(
    root: &Path,
    all_registered: bool,
    purge: bool,
) -> Result<serde_json::Value> {
    let (roots, profiles, agents) = service_profiles(root, all_registered)?;
    #[cfg(not(target_os = "macos"))]
    let _ = &agents;
    let mut locks = Vec::new();
    for root in &roots {
        if root.is_dir() {
            locks.push(control_lock(root).await?);
        }
    }
    // Validate registries before stopping anything. Never recursively erase an
    // arbitrary --data-dir, or orphaned directories whose vault is unknown.
    let mut registries = Vec::new();
    if purge {
        for root in &roots {
            if !root.exists() {
                continue;
            }
            let registry = shum_store::profiles::Profiles::new(root)?;
            let (_, registered) = registry.list()?;
            for (_, id) in profiles.iter().filter(|(path, _)| path == root) {
                if !registered.iter().any(|profile| &profile.id == id) {
                    bail!(
                        "{}",
                        crate::i18n::format(
                            "Профиль {id} отсутствует в реестре {}; полное удаление отменено",
                            &[("id", id.to_string()), ("0", format!("{}", root.display()))]
                        )
                    );
                }
            }
            registries.push((root, registry, registered));
        }
    }
    // A failed graceful stop aborts removal: don't erase a running bundle.
    for (root, id) in &profiles {
        crate::ipc::stop(root, id).await?;
    }
    // A registry may retain a profile with a missing directory after an
    // interrupted deletion. Include these entries when stopping services too.
    for (root, _, registered) in &registries {
        for profile in registered {
            if !profiles.contains(&((*root).clone(), profile.id.clone())) {
                crate::ipc::stop(root, &profile.id).await?;
            }
        }
    }
    #[cfg(target_os = "macos")]
    for agent in &agents {
        bootout(&agent.id)?;
        std::fs::remove_file(&agent.file)?;
    }
    #[cfg(target_os = "linux")]
    for (_, id) in &profiles {
        let config = directories::BaseDirs::new()
            .context(t("Нет каталога конфигурации"))?
            .config_dir()
            .join(format!("systemd/user/shum-{id}.service"));
        if config.exists() {
            run(Command::new("systemctl").args([
                "--user",
                "disable",
                "--now",
                &format!("shum-{id}.service"),
            ]))?;
            std::fs::remove_file(config)?;
            run(Command::new("systemctl").args(["--user", "daemon-reload"]))?;
        }
    }
    #[cfg(target_os = "windows")]
    for (_, id) in &profiles {
        let name = format!("Shum-{id}");
        if Command::new("schtasks")
            .args(["/Query", "/TN", &name])
            .output()?
            .status
            .success()
        {
            run(Command::new("schtasks").args(["/Delete", "/F", "/TN", &name]))?;
        }
    }
    let mut caches = 0;
    for root in &roots {
        caches += remove_caches(root)?;
    }
    if purge {
        let mut deleted = 0;
        for (_, registry, registered) in registries {
            for profile in registered {
                registry.delete(&profile.id).with_context(|| {
                    crate::i18n::format(
                        "Не удалось удалить профиль {}; оставшиеся данные сохранены",
                        &[("0", profile.id.to_string())],
                    )
                })?;
                deleted += 1;
            }
        }
        drop(locks);
        let mut remaining = Vec::new();
        for root in &roots {
            if !root.exists() {
                continue;
            }
            for name in [
                "profiles.json",
                "profiles.lock",
                "service-control.lock",
                "cli-settings.json",
            ] {
                let path = root.join(name);
                match std::fs::symlink_metadata(&path) {
                    Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
                        std::fs::remove_file(path)?;
                    }
                    Ok(_) => bail!(
                        "{}",
                        crate::i18n::format(
                            "Недействительный файл реестра: {}",
                            &[("0", format!("{}", path.display()))]
                        )
                    ),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            if std::fs::read_dir(root)?.next().is_none() {
                std::fs::remove_dir(root)?;
            } else {
                remaining.push(root.display().to_string());
            }
        }
        if !remaining.is_empty() {
            bail!(
                "{}",
                crate::i18n::format(
                    "Профили удалены, но посторонние файлы сохранены в: {}",
                    &[("0", remaining.join(", ").to_string())]
                )
            );
        }
        return Ok(
            serde_json::json!({"uninstalled": true, "purged": true, "profilesDeleted": deleted, "bundlesRemoved": caches}),
        );
    }
    Ok(
        serde_json::json!({"uninstalled": true, "profilesPreserved": profiles.len(), "bundlesRemoved": caches}),
    )
}

fn remove_caches(root: &Path) -> Result<usize> {
    let cache = root.join("services");
    if !cache.exists() {
        return Ok(0);
    }
    let meta = std::fs::symlink_metadata(&cache)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        bail!("{}", t("Недействительный каталог кэша служб"));
    }
    let mut removed = 0;
    for entry in std::fs::read_dir(&cache)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path();
        let app = if name == "Shum.app" {
            path.clone()
        } else if name.len() == 64 && name.bytes().all(|b| b.is_ascii_hexdigit()) {
            path.join("Shum.app")
        } else {
            continue;
        };
        if let Ok(meta) = std::fs::symlink_metadata(&app) {
            if meta.is_dir() && !meta.file_type().is_symlink() {
                std::fs::remove_dir_all(&app)?;
                removed += 1;
                if app != path && std::fs::read_dir(&path)?.next().is_none() {
                    std::fs::remove_dir(&path)?;
                }
            }
        }
    }
    if std::fs::read_dir(&cache)?.next().is_none() {
        std::fs::remove_dir(cache)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_agent_ids_match_profile_registry_format() {
        assert!(profile_id("60f55c0dc063911567469ec77f85cb83"));
        for id in [
            "f8067d32-e74e-4ea4-a362-e4f4a63f6c7c",
            "../data",
            "A",
            "60F55C0DC063911567469EC77F85CB83",
        ] {
            assert!(!profile_id(id));
        }
    }
    #[test]
    fn cache_removal_preserves_profile_data_and_unrelated_files() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let profile = root.join("f8067d32e74e4ea4a362e4f4a63f6c7c");
        std::fs::create_dir(&profile).unwrap();
        for name in ["keys.bin", "messages.sqlite", "profiles.json"] {
            std::fs::write(profile.join(name), b"preserve").unwrap();
        }
        let cache = root.join("services");
        let version = cache.join("a".repeat(64));
        std::fs::create_dir_all(version.join("Shum.app/Contents/MacOS")).unwrap();
        std::fs::write(version.join("keep.txt"), b"other file").unwrap();
        std::fs::create_dir_all(cache.join("Shum.app/Contents")).unwrap();
        #[cfg(unix)]
        {
            let external = root.join("external");
            std::fs::create_dir(&external).unwrap();
            std::fs::write(external.join("keep"), b"preserve").unwrap();
            std::os::unix::fs::symlink(&external, cache.join("b".repeat(64))).unwrap();
        }
        assert_eq!(remove_caches(root).unwrap(), 2);
        assert_eq!(remove_caches(root).unwrap(), 0);
        assert!(version.join("keep.txt").exists());
        for name in ["keys.bin", "messages.sqlite", "profiles.json"] {
            assert_eq!(std::fs::read(profile.join(name)).unwrap(), b"preserve");
        }
        #[cfg(unix)]
        assert!(root.join("external/keep").exists());
    }
}
