//! Local authenticated IPC. The service that owns a profile database lives in
//! Shum-Core (`shum_session::service`); the CLI starts it and words its errors.
use crate::i18n::t;
use crate::runtime::{describe, Failure, Request};
#[cfg(not(target_os = "macos"))]
use anyhow::Context;
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use shum_session::service;
use shum_store::profiles::{Profile, Profiles};
#[cfg(not(target_os = "macos"))]
use std::process::{Command as Process, Stdio};
use std::{fs::OpenOptions, path::Path, time::Duration};

/// A service error in the interface language.
fn worded(error: anyhow::Error) -> anyhow::Error {
    anyhow!("{}", describe(&Failure::from_error(error)))
}
pub fn select(profiles: &Profiles, selector: Option<&str>) -> Result<Profile> {
    let (selected, list) = profiles.list()?;
    let name = selector
        .or(selected.as_deref())
        .ok_or_else(|| anyhow!("{}", t("Профиля пока нет. Выполните shum init.")))?;
    list.into_iter()
        .find(|p| !p.deleting && p.id == name)
        .ok_or_else(|| {
            anyhow!(
                "{}",
                crate::i18n::format(
                    "Профиль не найден: {name}. Укажите ID из shum profile list.",
                    &[("name", name.to_string())]
                )
            )
        })
}
pub async fn request(root: &Path, id: &str, request: Request) -> Result<Value> {
    service::request(root, id, request).await.map_err(worded)
}
/// The `owner` a service reports. Services before this field were the CLI's.
const CLI_OWNER: &str = "cli";
fn owner(snapshot: &Value) -> &str {
    snapshot["owner"].as_str().unwrap_or(CLI_OWNER)
}
/// Another Shum app owns the profile: join it while the command contract
/// matches and never restart it, so its window keeps working.
fn join(snapshot: &Value) -> Result<()> {
    match snapshot["clientCommandVersion"].as_u64() {
        Some(version) if version == crate::runtime::COMMAND_SCHEMA_VERSION => Ok(()),
        Some(version) if version > crate::runtime::COMMAND_SCHEMA_VERSION => bail!(
            "{}",
            t("Обновите CLI: профиль открыт в более новом приложении Shum.")
        ),
        _ => bail!(
            "{}",
            t("Профиль открыт в старом приложении Shum. Обновите приложение или закройте его.")
        ),
    }
}
pub async fn ensure(root: &Path, id: &str) -> Result<()> {
    let _control = crate::service::control_lock(root).await?;
    let own = crate::identity::current()?.clone();
    let snapshot = request(root, id, Request::Snapshot).await.ok();
    if let Some(snapshot) = &snapshot {
        if owner(snapshot) != CLI_OWNER {
            return join(snapshot);
        }
        if snapshot["clientCommandVersion"] == crate::runtime::COMMAND_SCHEMA_VERSION
            && crate::identity::matches(snapshot, &own)
        {
            return Ok(());
        }
        if snapshot["clientCommandVersion"]
            .as_u64()
            .is_some_and(|v| v > crate::runtime::COMMAND_SCHEMA_VERSION)
        {
            bail!("{}", t("Обновите CLI: запущена более новая служба."));
        }
    }
    #[cfg(target_os = "macos")]
    let build = crate::macos::service_build(root)?;
    #[cfg(not(target_os = "macos"))]
    let build = own;
    if let Some(snapshot) = snapshot {
        match snapshot["clientCommandVersion"].as_u64() {
            Some(version)
                if version == crate::runtime::COMMAND_SCHEMA_VERSION
                    && crate::identity::matches(&snapshot, &build) =>
            {
                return Ok(())
            }
            Some(version) if version > crate::runtime::COMMAND_SCHEMA_VERSION => {
                bail!("{}", t("Обновите CLI: запущена более новая служба."))
            }
            // Gracefully checkpoint the old service before loading the new command contract.
            _ => stop(root, id).await?,
        }
    }
    stop(root, id).await?;
    if service::is_locked(root, id) {
        bail!("{}", t("Профиль заблокирован. Выполните shum unlock."));
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let log = options.open(root.join(id).join("daemon.log"))?;
    #[cfg(target_os = "macos")]
    {
        drop(log);
        if crate::service::is_installed(root, id)? {
            crate::service::install(root, id)?;
        } else {
            crate::macos::launch(root, id)?;
        }
    }
    #[cfg(not(target_os = "macos"))]
    let mut child = {
        let executable = std::env::current_exe()?;
        let mut command = Process::new(executable);
        command
            .arg("--data-dir")
            .arg(root)
            .arg("--profile")
            .arg(id)
            .args(["daemon", "--run"])
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
            command.creation_flags(0x00000008 | 0x08000000);
        }
        command
            .spawn()
            .context(t("Не удалось запустить службу Shum"))?
    };
    // On the first macOS launch, reading the profile can wait for the user
    // to approve Keychain access for the app's new code identity.
    let startup_seconds = if cfg!(target_os = "macos") { 60 } else { 10 };
    for _ in 0..startup_seconds * 10 {
        if let Ok(snapshot) = request(root, id, Request::Snapshot).await {
            // The app may have opened the profile while this service started.
            if owner(&snapshot) != CLI_OWNER {
                return join(&snapshot);
            }
            if snapshot["clientCommandVersion"] == crate::runtime::COMMAND_SCHEMA_VERSION
                && crate::identity::matches(&snapshot, &build)
            {
                return Ok(());
            }
            bail!(
                "{}",
                t("Версия службы не соответствует CLI. Перезапустите службу.")
            );
        }
        #[cfg(not(target_os = "macos"))]
        if child.try_wait()?.is_some() {
            // Another CLI may have won the exclusive profile lock while starting.
            tokio::time::sleep(Duration::from_millis(200)).await;
            if let Ok(snapshot) = request(root, id, Request::Snapshot).await {
                if crate::identity::matches(&snapshot, &build) {
                    return Ok(());
                }
            }
            bail!(
                "{}",
                crate::i18n::format(
                    "Служба завершилась. Диагностика: {}",
                    &[(
                        "0",
                        format!("{}", root.join(id).join("daemon.log").display())
                    )]
                )
            );
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("{}", crate::i18n::format("Служба не запустилась за {startup_seconds} секунд. Проверьте системный запрос Shum. Диагностика: {}", &[("startup_seconds", format!("{}", startup_seconds)), ("0", format!("{}", root.join(id).join("daemon.log").display()))]))
}
pub async fn stop(root: &Path, id: &str) -> Result<()> {
    service::stop(root, id).await.map_err(worded)
}
async fn shutdown_signal() {
    // Detached Windows processes do not always have a console control handler.
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}
pub async fn serve(root: &Path, id: &str) -> Result<()> {
    let mut client = serde_json::Map::new();
    client.insert("owner".into(), serde_json::json!(CLI_OWNER));
    client.insert("version".into(), serde_json::json!(env!("SHUM_VERSION")));
    client.insert(
        "build".into(),
        serde_json::to_value(crate::identity::current()?)?,
    );
    service::serve(root, id, client, shutdown_signal())
        .await
        .map_err(worded)
}
