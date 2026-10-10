use ratatui::{backend::TestBackend, Terminal};
use ratatui_image::picker::Picker;
use serde_json::json;
use shum_cli::{
    display::Display,
    ui::{draw, Pictures, View},
};

#[test]
fn warp_profile_modal_keeps_its_own_images_and_hides_chat_images() {
    let display = Display::for_terminal("WarpTerminal", "xterm-256color", "", false, false);
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(display.images);
    let mut pictures = Pictures::with_display(picker, display);
    let snapshot = json!({"profile":{"id":"first"},"card":{"name":"Same name","avatarSeed":42},"contacts":[{"id":"peer","phase":"accepted","card":{"name":"Peer","avatarSeed":123}}]});
    let mut view = View::chat("peer");
    view.profiles = Some(
        serde_json::from_value(json!([
            {"id":"first","name":"Same name","ownerId":"a","keyBackend":"file"},
            {"id":"second","name":"Same name","ownerId":"b","keyBackend":"file"}
        ]))
        .unwrap(),
    );
    view.profile_details.insert(
        "second".into(),
        json!({"card":{"avatarSeed":123},"chatCount":2}),
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
    for selected in [0, 1, 2, 0] {
        view.profile_selected = selected;
        terminal
            .draw(|frame| draw(frame, &snapshot, &mut view, &mut pictures, false))
            .unwrap();
        let images = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.symbol().contains("\x1b_Ga=T,"))
            .collect::<Vec<_>>();
        assert_eq!(
            images.len(),
            2,
            "only the two profile avatars should be visible"
        );
        for image in images {
            assert!(
                image.symbol().contains("c=6,r=3,"),
                "profile thumbnails stay inside their rows"
            );
        }
    }
    // A second modal must cover the profile PNGs as well as the chat PNGs.
    view.help = true;
    terminal
        .draw(|frame| draw(frame, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    assert!(terminal
        .backend()
        .buffer()
        .content
        .iter()
        .all(|c| !c.symbol().contains("\x1b_Ga=T,")));
}
