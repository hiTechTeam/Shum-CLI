use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, style::Color, Terminal};
use ratatui_image::picker::Picker;
use serde_json::{json, Value};
use shum_cli::{
    display::Colors,
    runtime::Request,
    ui::{draw, handle_key, Action, Pictures, View},
};
use shum_core::packet::ReactionKind;

fn snapshot() -> Value {
    json!({"profile":{"ownerId":"me"},"contacts":[{"id":"peer","card":{"name":"Аня"},"phase":"accepted"}],"messages":[
        {"id":"one","contactID":"peer","text":"First message","outgoing":false},
        {"id":"foreign","contactID":"other","text":"Another chat"},
        {"id":"two","contactID":"peer","text":"Second message","outgoing":true}
    ],"reactions":[]})
}
fn key(view: &mut View, snapshot: &Value, code: KeyCode) -> anyhow::Result<Action> {
    handle_key(view, snapshot, KeyEvent::new(code, KeyModifiers::NONE))
}
fn open(view: &mut View, snapshot: &Value) {
    view.input = "/react".into();
    assert!(matches!(
        key(view, snapshot, KeyCode::Enter).unwrap(),
        Action::None
    ));
    assert!(view.input.is_empty());
}
fn reaction(action: Action, id: &str, expected: ReactionKind) {
    match action {
        Action::Request(Request::Reaction { message, reaction }) => {
            assert_eq!(message, id);
            assert_eq!(reaction, expected);
        }
        _ => panic!("expected reaction request"),
    }
}
#[test]
fn picks_message_then_pixel_reaction_without_an_id() {
    let snapshot = snapshot();
    let mut view = View::chat("peer");
    open(&mut view, &snapshot);
    key(&mut view, &snapshot, KeyCode::Up).unwrap();
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    key(&mut view, &snapshot, KeyCode::Down).unwrap();
    reaction(
        key(&mut view, &snapshot, KeyCode::Enter).unwrap(),
        "one",
        ReactionKind::Like,
    );
}
#[test]
fn live_updates_never_shift_selected_message_to_another_id() {
    let mut snapshot = snapshot();
    let mut view = View::chat("peer");
    open(&mut view, &snapshot);
    snapshot["messages"]
        .as_array_mut()
        .unwrap()
        .insert(0, json!({"id":"older","contactID":"peer","text":"Old"}));
    snapshot["messages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"new","contactID":"peer","text":"New"}));
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    reaction(
        key(&mut view, &snapshot, KeyCode::Enter).unwrap(),
        "two",
        ReactionKind::Heart,
    );
}
#[test]
fn deleted_message_is_not_replaced_silently() {
    let mut snapshot = snapshot();
    let mut view = View::chat("peer");
    open(&mut view, &snapshot);
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    snapshot["messages"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["id"] != "two");
    assert!(key(&mut view, &snapshot, KeyCode::Enter)
        .err()
        .unwrap()
        .to_string()
        .contains("удалено"));
    key(&mut view, &snapshot, KeyCode::Esc).unwrap();
    key(&mut view, &snapshot, KeyCode::Home).unwrap();
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    reaction(
        key(&mut view, &snapshot, KeyCode::Enter).unwrap(),
        "one",
        ReactionKind::Heart,
    );
}
#[test]
fn escape_cancels_without_sending_and_global_exit_still_works() {
    let snapshot = snapshot();
    let mut view = View::chat("peer");
    open(&mut view, &snapshot);
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    assert!(matches!(
        key(&mut view, &snapshot, KeyCode::Esc).unwrap(),
        Action::None
    ));
    assert!(matches!(
        key(&mut view, &snapshot, KeyCode::Esc).unwrap(),
        Action::None
    ));
    key(&mut view, &snapshot, KeyCode::Char('x')).unwrap();
    assert_eq!(view.input, "x");
    open(&mut view, &snapshot);
    assert!(matches!(
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)
        )
        .unwrap(),
        Action::Quit
    ));
}
#[test]
fn picker_is_modal_and_existing_reaction_can_be_toggled() {
    let mut snapshot = snapshot();
    snapshot["reactions"] = json!([{"messageID":"two","personID":"me","mark":{"reaction":"fire"}}]);
    let mut view = View::chat("peer");
    open(&mut view, &snapshot);
    assert!(matches!(
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)
        )
        .unwrap(),
        Action::None
    ));
    key(&mut view, &snapshot, KeyCode::Enter).unwrap();
    reaction(
        key(&mut view, &snapshot, KeyCode::Enter).unwrap(),
        "two",
        ReactionKind::Fire,
    );
}
#[test]
fn empty_chat_and_missing_chat_have_actionable_errors() {
    let mut snapshot = snapshot();
    let mut view = View::default();
    view.command_mode = true;
    view.input = "/react".into();
    assert!(key(&mut view, &snapshot, KeyCode::Enter).is_err());
    snapshot["messages"] = json!([]);
    view = View::chat("peer");
    view.input = "/react".into();
    assert!(key(&mut view, &snapshot, KeyCode::Enter).is_err());
}
#[test]
fn explicit_reaction_command_is_preserved() {
    let mut view = View::chat("peer");
    view.input = "/react one horror".into();
    reaction(
        key(&mut view, &snapshot(), KeyCode::Enter).unwrap(),
        "one",
        ReactionKind::Horror,
    );
}
#[test]
fn all_pixel_reactions_render_in_rgb_indexed_and_ascii_after_resize() {
    let snapshot = snapshot();
    for colors in [Colors::Rgb, Colors::Indexed] {
        for ascii in [false, true] {
            let mut view = View::chat("peer");
            open(&mut view, &snapshot);
            key(&mut view, &snapshot, KeyCode::Enter).unwrap();
            let mut pictures = Pictures::with_colors(Picker::halfblocks(), colors);
            for (width, height) in [(80, 24), (50, 18), (35, 12), (120, 40)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                for _ in 0..8 {
                    terminal
                        .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, ascii))
                        .unwrap();
                    let buffer = terminal.backend().buffer();
                    let body = buffer
                        .content
                        .iter()
                        .map(|c| c.symbol())
                        .collect::<String>();
                    assert!(body.contains("Выберите реакцию"));

                    if !ascii && height >= 18 {
                        let colored = buffer.content.iter().filter(|c| match colors {
                            Colors::Rgb => matches!(c.bg, Color::Rgb(r,g,b) if r > 50 && (r != g || g != b)),
                            Colors::Indexed => matches!(c.bg, Color::Indexed(i) if i > 16 && i < 232),
                        }).count();
                        assert!(colored > 10, "pixel artwork must be visible");
                    }
                    key(&mut view, &snapshot, KeyCode::Down).unwrap();
                }
            }
        }
    }
}

#[test]
fn ctrl_r_opens_picker_in_chat_and_preserves_draft_on_cancel() {
    let snapshot = snapshot();
    for letter in ['r', 'к'] {
        let mut view = View::chat("peer");
        view.input = "Unsent draft / {name}".into();
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL),
        )
        .unwrap();
        key(&mut view, &snapshot, KeyCode::Up).unwrap();
        key(&mut view, &snapshot, KeyCode::Enter).unwrap();
        key(&mut view, &snapshot, KeyCode::Esc).unwrap();
        key(&mut view, &snapshot, KeyCode::Esc).unwrap();
        assert_eq!(view.input, "Unsent draft / {name}");
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL),
        )
        .unwrap();
        key(&mut view, &snapshot, KeyCode::Enter).unwrap();
        reaction(
            key(&mut view, &snapshot, KeyCode::Enter).unwrap(),
            "two",
            ReactionKind::Heart,
        );
    }
}

#[test]
fn ctrl_r_requires_a_chat_and_does_not_open_over_another_modal() {
    let snapshot = snapshot();
    let ctrl_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert!(handle_key(&mut View::default(), &snapshot, ctrl_r).is_err());
    let mut view = View::chat("peer");
    view.help = true;
    view.input = "draft".into();
    handle_key(&mut view, &snapshot, ctrl_r).unwrap();
    assert!(view.help);
    assert_eq!(view.input, "draft");
    key(&mut view, &snapshot, KeyCode::Esc).unwrap();
    assert!(!view.help);
}
