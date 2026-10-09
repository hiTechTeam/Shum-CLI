use shum_store::{profiles::Profiles, vault::KeyMode};
use std::{path::Path, process::Command};

fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_shum"))
        .arg("--data-dir")
        .arg(root)
        .arg("--json")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn purge_requires_explicit_confirmation_and_is_repeatable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    let profiles = Profiles::new(&root).unwrap();
    let one = profiles.create("One", KeyMode::File).unwrap();
    let two = profiles.create("Two", KeyMode::File).unwrap();
    let keys = root.join(&one.id).join("keys.bin");
    let original = std::fs::read(&keys).unwrap();
    for args in [
        vec!["daemon", "--purge", "--confirm", "DELETE"],
        vec!["daemon", "--uninstall", "--purge"],
        vec!["daemon", "--uninstall", "--purge", "--confirm", "NO"],
        vec!["daemon", "--uninstall", "--confirm", "DELETE"],
    ] {
        assert!(!cli(&root, &args).status.success());
        assert_eq!(std::fs::read(&keys).unwrap(), original);
        assert_eq!(profiles.list().unwrap().1.len(), 2);
    }
    assert!(cli(&root, &["daemon", "--uninstall"]).status.success());
    assert_eq!(std::fs::read(&keys).unwrap(), original);
    assert!(root.join(&two.id).join("messages.sqlite").exists());
    let cache = root.join("services/Shum.app/Contents");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("Info.plist"), b"cached app").unwrap();
    let result = cli(
        &root,
        &["daemon", "--uninstall", "--purge", "--confirm", "DELETE"],
    );
    assert!(result.status.success(), "{:?}", result);
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(json["profilesDeleted"], 2);
    assert_eq!(json["bundlesRemoved"], 1);
    assert!(!root.exists());
    let repeat = cli(
        &root,
        &["daemon", "--uninstall", "--purge", "--confirm", "DELETE"],
    );
    assert!(repeat.status.success(), "{:?}", repeat);
    assert!(!root.exists());
}

#[tokio::test]
async fn purge_aborts_for_running_database_and_preserves_unrelated_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    let profiles = Profiles::new(&root).unwrap();
    let profile = profiles.create("One", KeyMode::File).unwrap();
    let open = profiles.open(Some(&profile.id)).unwrap();
    // Profile deletion cannot acquire the database lock held by another process.
    assert!(shum_cli::service::purge(&root, false).await.is_err());
    assert!(root.join(&profile.id).join("keys.bin").exists());
    assert_eq!(profiles.list().unwrap().1.len(), 1);
    drop(open);
    let unrelated = root.join("unrelated.txt");
    std::fs::write(&unrelated, b"preserve").unwrap();
    assert!(shum_cli::service::purge(&root, false).await.is_err());
    assert_eq!(std::fs::read(unrelated).unwrap(), b"preserve");
    assert!(!root.join(&profile.id).exists());
}

#[cfg(unix)]
#[tokio::test]
async fn purge_rejects_profile_symlinks_without_erasing_the_target() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    let profiles = Profiles::new(&root).unwrap();
    let profile = profiles.create("One", KeyMode::File).unwrap();
    let external = temporary.path().join("external");
    std::fs::rename(root.join(&profile.id), &external).unwrap();
    std::os::unix::fs::symlink(&external, root.join(&profile.id)).unwrap();
    assert!(shum_cli::service::purge(&root, false).await.is_err());
    assert!(external.join("keys.bin").exists());
    assert!(external.join("messages.sqlite").exists());
    assert_eq!(profiles.list().unwrap().1.len(), 1);
}
