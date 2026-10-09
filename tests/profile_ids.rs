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
fn cli_requires_ids_to_select_and_delete_equal_names() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("data");
    let profiles = Profiles::new(&root).unwrap();
    let one = profiles.create("Same", KeyMode::File).unwrap();
    let two = profiles.create("Same", KeyMode::File).unwrap();
    for args in [
        vec!["profile", "use", "Same"],
        vec!["profile", "delete", "Same", "--confirm", "Same"],
        vec!["profile", "delete", &one.id, "--confirm", "wrong"],
        vec!["profile", "delete"],
    ] {
        assert!(!cli(&root, &args).status.success());
        assert_eq!(profiles.list().unwrap().1.len(), 2);
    }
    assert!(cli(&root, &["profile", "use", &two.id]).status.success());
    assert_eq!(profiles.list().unwrap().0.as_deref(), Some(two.id.as_str()));
    assert!(
        cli(&root, &["profile", "delete", &one.id, "--confirm", "Same"])
            .status
            .success()
    );
    assert_eq!(profiles.open(None).unwrap().profile.id, two.id);
    assert!(!root.join(one.id).exists());
    assert!(root.join(&two.id).join("keys.bin").exists());
    assert!(
        cli(&root, &["profile", "delete", &two.id, "--confirm", "Same"])
            .status
            .success()
    );
}
