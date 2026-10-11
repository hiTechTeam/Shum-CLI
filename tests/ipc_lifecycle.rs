use std::fs;

#[tokio::test]
async fn installer_refresh_leaves_inactive_and_stale_profiles_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let id = "f8067d32e74e4ea4a362e4f4a63f6c7c";
    let profile = dir.path().join(id);
    fs::create_dir(&profile).unwrap();
    // Unreadable placeholder keys and no database: refresh must not open them.
    fs::write(profile.join("keys.bin"), b"do not open").unwrap();
    let result = shum_cli::service::refresh(dir.path(), false).await.unwrap();
    assert_eq!(result["activeProfilesRefreshed"], 0);
    fs::write(profile.join("daemon.json"), b"stale endpoint").unwrap();
    let result = shum_cli::service::refresh(dir.path(), false).await.unwrap();
    assert_eq!(result["activeProfilesRefreshed"], 0);
    assert_eq!(fs::read(profile.join("keys.bin")).unwrap(), b"do not open");
    assert!(!profile.join("messages.sqlite").exists());
}

/// A loopback service that reports `owner` and `version` and records every
/// request it receives.
async fn fake_service(
    root: &std::path::Path,
    id: &str,
    owner: &'static str,
    version: u64,
) -> std::sync::Arc<std::sync::Mutex<Vec<String>>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = root.join(id).join("daemon.json");
    fs::write(
        &endpoint,
        serde_json::json!({
            "port": listener.local_addr().unwrap().port(),
            "token": "00".repeat(32),
            "pid": std::process::id(),
        })
        .to_string(),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&endpoint, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = requests.clone();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut token = [0; 32];
            stream.read_exact(&mut token).await.unwrap();
            let length = stream.read_u32().await.unwrap() as usize;
            let mut bytes = vec![0; length];
            stream.read_exact(&mut bytes).await.unwrap();
            let request: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let name = request["command"].as_str().unwrap_or_default().to_owned();
            seen.lock().unwrap().push(name);
            let reply = serde_json::json!({"ok": {
                "owner": owner,
                "clientCommandVersion": version,
            }})
            .to_string();
            stream.write_u32(reply.len() as u32).await.unwrap();
            stream.write_all(reply.as_bytes()).await.unwrap();
        }
    });
    requests
}

#[tokio::test]
async fn cli_joins_a_profile_open_in_the_app_and_never_stops_it() {
    let current = shum_cli::runtime::COMMAND_SCHEMA_VERSION;
    for (version, joins) in [(current, true), (current + 1, false), (current - 1, false)] {
        let dir = tempfile::tempdir().unwrap();
        let id = "f8067d32e74e4ea4a362e4f4a63f6c7c";
        fs::create_dir(dir.path().join(id)).unwrap();
        let requests = fake_service(dir.path(), id, "desktop", version).await;
        let result = shum_cli::ipc::ensure(dir.path(), id).await;
        assert_eq!(result.is_ok(), joins, "version {version}: {result:?}");
        let requests = requests.lock().unwrap();
        assert!(
            requests.iter().all(|r| r == "snapshot"),
            "version {version}: {requests:?}"
        );
        assert!(dir.path().join(id).join("daemon.json").exists());
    }
}
