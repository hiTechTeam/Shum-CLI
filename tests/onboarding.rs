use shum_cli::onboarding::{create, initial_seed, prepare, Settings};
use shum_store::{
    profiles::Profiles,
    vault::{KeyMode, ProfileKeys},
};
#[test]
fn confirmed_avatar_and_prepared_keys_survive_reopening_with_equal_display_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("profiles");
    let settings = Settings {
        bluetooth: false,
        relays: vec!["ws://127.0.0.1:9".into()],
        push_url: None,
    };
    let keys = ProfileKeys::generate().unwrap();
    let owner = keys.owner_id();
    let created = create(&root, "Имя", Some(42), KeyMode::File, &settings, keys).unwrap();
    assert_eq!(created.card.id(), owner);
    assert_eq!(created.card.avatar_seed, Some(42));
    let profiles = Profiles::new(&root).unwrap();
    let open = profiles.open(Some(&created.profile.id)).unwrap();
    assert_eq!(open.store.state()["ownProfileCard"]["avatarSeed"], 42);
    assert_eq!(open.keys.owner_id(), owner);
    drop(open);
    let duplicate = create(
        &root,
        "Имя",
        Some(99),
        KeyMode::File,
        &settings,
        ProfileKeys::generate().unwrap(),
    )
    .unwrap();
    assert_ne!(duplicate.profile.id, created.profile.id);
    assert_ne!(duplicate.profile.owner_id, created.profile.owner_id);
    assert!(create(
        &root,
        " ",
        Some(99),
        KeyMode::File,
        &settings,
        ProfileKeys::generate().unwrap()
    )
    .is_err());
    let (selected, list) = profiles.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(selected.as_deref(), Some(duplicate.profile.id.as_str()));
    profiles.delete(&created.profile.id).unwrap();
    assert_eq!(
        profiles.open(None).unwrap().profile.id,
        duplicate.profile.id
    );
    profiles.delete(&duplicate.profile.id).unwrap();
}

#[test]
fn preparation_precedes_profile_publication_and_default_seed_uses_signing_key() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("data");
    let mut stages = Vec::new();
    let keys = prepare(&root, |stage| stages.push(stage)).unwrap();
    assert_eq!(stages, [0, 1, 2, 3, 4]);
    let expected = initial_seed(&keys);
    assert_eq!(
        expected,
        shum_core::crypto::avatar_seed(&keys.signing.ed_public())
    );
    assert!(Profiles::new(&root).unwrap().list().unwrap().1.is_empty());
    assert!(std::fs::read_dir(&root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".registration-")
    }));
    let settings = Settings {
        bluetooth: false,
        relays: vec!["ws://127.0.0.1:9".into()],
        push_url: None,
    };
    let created = create(&root, "Test", None, KeyMode::File, &settings, keys).unwrap();
    assert_eq!(created.card.avatar_seed, Some(expected));
    created.card.validate().unwrap();
    Profiles::new(&root)
        .unwrap()
        .delete(&created.profile.id)
        .unwrap();
}

#[test]
fn security_screen_shows_real_progress_then_name_before_avatar() {
    use ratatui::{backend::TestBackend, Terminal};
    use ratatui_image::picker::Picker;
    use shum_cli::{
        onboarding::{draw, draw_preparation, Step, Wizard},
        ui::Pictures,
    };
    for (width, height) in [(35, 18), (80, 32), (140, 45)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut pictures = Pictures::new(Picker::halfblocks());
        for completed in 0..=4 {
            terminal
                .draw(|frame| draw_preparation(frame, completed, &mut pictures, false))
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(text.contains("Криптографические ключи"));
            assert_eq!(text.matches('✓').count(), completed);
            assert!(text.contains(&format!("{}%", completed * 25)));
            assert!(!text.contains("? Имя"));
            assert!(!text.contains("Аватар"));
        }
        let wizard = Wizard {
            step: Step::Name,
            name: "Test".into(),
            seed: 42,
            error: String::new(),
            file_keys: true,
        };
        terminal
            .draw(|frame| draw(frame, &wizard, &mut pictures, false))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Ключ профиля создан."));
        assert!(text.contains("? Имя"));
        assert!(text.contains("Test"));
        assert!(!text.contains("Аватар"));
    }
}
