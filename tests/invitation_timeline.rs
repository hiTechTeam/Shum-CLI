use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use ratatui_image::picker::Picker;
use serde_json::json;
use shum_cli::{
    invitations,
    runtime::Request,
    ui::{draw, handle_key, Action, Pictures, View},
};

#[test]
fn invitation_events_render_in_history_and_cannot_receive_reactions() {
    let mut snapshot = json!({"contacts":[{"id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","card":{"name":"Peer"},"phase":"declinedLocally"}],
        "messages":[],"events":[
            {"id":"i1","contactID":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","timestamp":1,"kind":"invitationReceived"},
            {"id":"i2","contactID":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","timestamp":2,"kind":"invitationDeclinedLocally"},
            {"id":"other","contactID":"other","timestamp":3,"kind":"invitationSent"}]});
    let mut view = View::chat("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    let mut terminal = Terminal::new(TestBackend::new(150, 35)).unwrap();
    let mut pictures = Pictures::new(Picker::halfblocks());
    terminal
        .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    let output: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(output.contains("Получено приглашение в чат."));
    assert!(output.contains("Вы отклонили приглашение."));
    assert!(output.contains("/accept принять, если передумали"));
    assert!(!output.contains("Приглашение отправлено."));
    for (command, expected) in [
        ("/accept", "accept"),
        ("/decline", "decline"),
        ("/invite", "invite"),
    ] {
        view.input = command.into();
        let action = handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        )
        .unwrap();
        match (action, expected) {
            (Action::Request(Request::Accept { contact }), "accept")
            | (Action::Request(Request::Decline { contact }), "decline")
            | (Action::Request(Request::Invite { contact }), "invite") => {
                assert_eq!(
                    contact,
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                )
            }
            _ => panic!("incorrect invitation command"),
        }
    }
    // Timeline events must never be offered as reaction targets, even in accepted chats.
    snapshot["contacts"][0]["phase"] = json!("accepted");
    assert!(handle_key(
        &mut view,
        &snapshot,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
    )
    .is_err());
    snapshot["messages"] = json!([{"id":"m","contactID":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","timestamp":2,"text":"hello"}]);
    let items = invitations::timeline(
        &snapshot,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], "i1");
    assert_eq!(items[2]["id"], "m");
}
