use serde_json::{json, Value};
use shum_cli::i18n;
use std::{path::Path, process::Command};
fn cli(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_shum"))
        .env_remove("SHUM_LANG")
        .env("LC_ALL", "C")
        .args(["--data-dir"])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn locale_normalization_and_interpolation_preserve_user_content() {
    for (input, expected) in [
        ("ru_RU.UTF-8", "ru"),
        ("pt_PT.UTF-8", "pt-BR"),
        ("zh_CN", "zh-Hans"),
        ("en-GB", "en"),
        ("C.UTF-8", "en"),
        ("ar_SA", "ar"),
        ("POSIX", "en"),
    ] {
        assert_eq!(i18n::normalize(input), Some(expected));
    }
    assert_eq!(i18n::normalize("de_DE"), None);
    let values = [
        ("name", "{} {name} العربية 日本語".into()),
        ("0", "first".into()),
        ("1", "second".into()),
    ];
    assert_eq!(
        i18n::interpolate("{{{name}}}: {} / {}", &values),
        "{{} {name} العربية 日本語}: first / second"
    );
    assert_eq!(
        i18n::interpolate("{1} {0} {1}", &values),
        "second first second"
    );
}
#[test]
fn all_languages_have_localized_help_and_preserve_commands() {
    let temp = tempfile::tempdir().unwrap();
    for (code, _) in i18n::LANGUAGES {
        let output = cli(temp.path(), &["--lang", code, "--ascii", "--help"]);
        assert!(
            output.status.success(),
            "{code}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            text.contains(i18n::translate(code, "Мессенджер без номера телефона")),
            "{code}"
        );
        assert!(text.contains("--lang") && text.contains("language") && text.contains("daemon"));
        if code != "ru" {
            assert!(
                !text.chars().any(|c| ('А'..='я').contains(&c)),
                "unexpected Russian in {code}: {text}"
            );
        }
    }
    assert!(!temp.path().join("profiles.json").exists());
}
#[test]
fn language_precedence_persistence_and_json_are_stable() {
    let temp = tempfile::tempdir().unwrap();
    let value = |output: std::process::Output| -> Value {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    assert_eq!(
        value(cli(temp.path(), &["--json", "language"]))["language"],
        "en"
    );
    assert_eq!(
        value(cli(temp.path(), &["--json", "language", "ja"]))["language"],
        "ja"
    );
    assert_eq!(
        value(cli(temp.path(), &["--json", "language"]))["language"],
        "ja"
    );
    assert_eq!(
        value(cli(temp.path(), &["--lang=fr", "--json", "language"]))["language"],
        "fr"
    );
    assert_eq!(
        value(cli(temp.path(), &["--json", "language"]))["language"],
        "ja"
    );
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_shum"))
            .env("SHUM_LANG", "es")
            .env("LC_ALL", "ru_RU.UTF-8")
            .args(["--data-dir"])
            .arg(temp.path())
            .args(["--json", "language"])
            .args(arguments)
            .output()
            .unwrap()
    };
    assert_eq!(value(run(&[]))["language"], "es");
    assert_eq!(value(run(&["--lang", "ko"]))["language"], "ko");
    assert_eq!(value(run(&["--lang", "system"]))["language"], "ru");
    let bad = cli(temp.path(), &["--json", "language", "xx"]);
    assert!(!bad.status.success());
    assert!(serde_json::from_slice::<Value>(&bad.stdout).unwrap()["error"].is_string());
    assert_eq!(
        value(cli(temp.path(), &["--json", "language"]))["language"],
        "ja"
    );
    assert_eq!(
        value(cli(temp.path(), &["--json", "language", "system"]))["language"],
        "en"
    );
    assert!(!temp.path().join("profiles.json").exists());
}
#[test]
fn localized_chat_and_picker_keep_names_and_message_text_literal() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};
    use ratatui_image::picker::Picker;
    use shum_cli::ui::{draw, handle_key, Pictures, View};
    let id = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let snapshot = json!({"profile":{"ownerId":"own"},"card":{"name":"Me"},
        "contacts":[{"id":id,"phase":"accepted","card":{"name":"User {name}","avatarSeed":42}}],
        "messages":[{"id":"one","contactID":id,"text":"печатает… {name} 🔥 العربية"}]});
    for (code, _) in i18n::LANGUAGES {
        i18n::select(code).unwrap();
        for width in [35, 80, 160] {
            let mut view = View::chat(id);
            let mut pictures = Pictures::new(Picker::halfblocks());
            let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, true))
                .unwrap();
            let text: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(text.contains("User {name}"), "{code}/{width}");
            assert!(text.contains("печатает… {name}"), "{code}/{width}");
            handle_key(
                &mut view,
                &snapshot,
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
            )
            .unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, true))
                .unwrap();
            handle_key(
                &mut view,
                &snapshot,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            )
            .unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, true))
                .unwrap();
        }
    }
    i18n::select("ru").unwrap();
}
