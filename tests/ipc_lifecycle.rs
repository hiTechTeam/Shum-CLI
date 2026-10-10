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
