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

#[test]
fn ceremony_waits_for_operations_and_animates_between_checkmarks() {
    use shum_cli::onboarding::CreationAnimation;
    use std::time::Duration;
    let mut animation = CreationAnimation::default();
    animation.advance(0, Duration::from_secs(10));
    assert_eq!(animation.completed(), 0);
    assert!(animation.percent() < 25);
    assert!(!animation.finished());
    animation.advance(1, Duration::from_millis(1));
    assert_eq!(animation.completed(), 1);
    animation.advance(1, Duration::from_secs(10));
    assert_eq!(animation.completed(), 1);
    assert!(animation.percent() < 50);
    animation.advance(2, Duration::from_secs(10));
    assert_eq!(animation.completed(), 2);
    assert!(animation.percent() < 75);
    animation.advance(3, Duration::from_secs(10));
    assert_eq!(animation.completed(), 3);
    assert!(animation.percent() < 100);
    assert!(!animation.finished());
    animation.advance(4, Duration::from_millis(1));
    assert_eq!(animation.completed(), 4);
    assert_eq!(animation.percent(), 100);
    assert!(!animation.finished());
    animation.advance(4, Duration::from_millis(450));
    assert!(animation.finished());

    // Fast crypto must still leave enough frames to perceive key assembly,
    // smooth progress and four distinct checkmarks before the Name prompt.
    let mut fast = CreationAnimation::default();
    let mut percentages = std::collections::BTreeSet::new();
    let mut stages = std::collections::BTreeSet::new();
    for _ in 0..200 {
        fast.advance(4, Duration::from_millis(33));
        percentages.insert(fast.percent());
        stages.insert(fast.completed());
    }
    assert!(percentages.len() > 90);
    assert_eq!(stages, (0..=4).collect());
    assert!(fast.finished());
}

#[test]
fn key_assembles_in_visible_frames_without_prompting_for_name() {
    use ratatui::{backend::TestBackend, style::Color, Terminal};
    use ratatui_image::picker::Picker;
    use shum_cli::{
        onboarding::{draw_animated_preparation, CreationAnimation},
        ui::Pictures,
    };
    use std::time::Duration;
    for (width, height) in [(80, 32), (140, 45)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut pictures =
            Pictures::with_colors(Picker::halfblocks(), shum_cli::display::Colors::Rgb);
        let mut animation = CreationAnimation::default();
        let mut drawings = Vec::new();
        for millis in [0, 700, 800, 1500, 1500, 1500, 450] {
            animation.advance(4, Duration::from_millis(millis));
            terminal
                .draw(|frame| draw_animated_preparation(frame, &animation, &mut pictures, false))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text = buffer
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(!text.contains("? Имя"));
            assert!(!text.contains("Аватар"));
            assert_eq!(text.matches('✓').count(), animation.completed());
            let cells = (6..19)
                .flat_map(|y| (4..36).map(move |x| (x, y)))
                .map(|point| {
                    let cell = &buffer[point];
                    assert_eq!(cell.symbol(), " ");
                    cell.bg
                })
                .collect::<Vec<_>>();
            drawings.push(cells);
        }
        assert!(drawings[0].iter().all(|c| *c == Color::Rgb(10, 13, 11)));
        assert!(drawings.windows(2).all(|frames| frames[0] != frames[1]));
        assert!(animation.finished());
    }
}
