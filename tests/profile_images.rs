use ratatui::{backend::TestBackend, Terminal};
use ratatui_image::picker::Picker;
use serde_json::json;
use shum_cli::{
    display::Display,
    ui::{draw, Pictures, View},
};

#[test]
fn warp_profile_modal_keeps_its_own_and_uncovered_chat_images() {
    let display = Display::for_terminal("WarpTerminal", "xterm-256color", "", false, false);
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(display.images);
    let mut pictures = Pictures::with_display(picker, display);
    let snapshot = json!({"profile":{"id":"first"},"card":{"name":"Same name","avatarSeed":42},"contacts":[{"id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","phase":"accepted","card":{"name":"Peer","avatarSeed":123}}]});
    let mut view = View::chat("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
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
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
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
            4,
            "both profile thumbnails and both uncovered chat avatars stay visible"
        );
        for row in [19, 23] {
            assert!(
                images
                    .iter()
                    .any(|image| image.symbol().contains(&format!("\x1b[{row};57H"))
                        && image.symbol().contains("c=6,r=3,")),
                "profile thumbnail stays inside its row"
            );
        }
    }
    // A second modal covers only the images under its own rectangle.
    view.help = true;
    terminal
        .draw(|frame| draw(frame, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    assert_eq!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|c| c.symbol().contains("\x1b_Ga=T,"))
            .count(),
        2,
        "help covers profile thumbnails but leaves both chat avatars outside it"
    );
}

#[test]
fn reaction_picker_keeps_uncovered_avatars_and_restores_them_after_closing() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{buffer::Buffer, layout::Rect};
    use shum_cli::ui::handle_key;
    fn positions(buffer: &Buffer) -> Vec<Rect> {
        buffer
            .content
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let symbol = c.symbol();
                if !symbol.contains("\x1b_Ga=T,") {
                    return None;
                }
                let dimension = |key: &str| -> u16 {
                    symbol
                        .split_once(key)
                        .unwrap()
                        .1
                        .split(',')
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap()
                };
                Some(Rect::new(
                    i as u16 % buffer.area.width,
                    i as u16 / buffer.area.width,
                    dimension("c="),
                    dimension("r="),
                ))
            })
            .collect()
    }
    let display = Display::for_terminal("WarpTerminal", "xterm-256color", "", false, false);
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(display.images);
    let mut pictures = Pictures::with_display(picker, display);
    let snapshot = json!({"profile":{"ownerId":"own"},"contacts":[
        {"id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","phase":"accepted","card":{"name":"Peer","avatarSeed":123}},
        {"id":"other","phase":"accepted","card":{"name":"Other","avatarSeed":42}}
    ],"messages":[{"id":"one","contactID":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","text":"Hello"}]});
    for (width, height) in [(160, 50), (100, 24)] {
        let mut view =
            View::chat("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
            .unwrap();
        let before = positions(terminal.backend().buffer());
        assert_eq!(before.len(), 3);
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        )
        .unwrap();
        let popup = Rect::new((width - 72) / 2, (height - 22) / 2, 72, 22);
        let outside: Vec<_> = before
            .iter()
            .copied()
            .filter(|r| r.intersection(popup).is_empty())
            .collect();
        assert!(!outside.is_empty());
        if width == 100 {
            assert!(outside.len() < before.len());
        }
        for code in [KeyCode::Up, KeyCode::Enter, KeyCode::Down, KeyCode::Esc] {
            handle_key(
                &mut view,
                &snapshot,
                KeyEvent::new(code, KeyModifiers::NONE),
            )
            .unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
                .unwrap();
            assert_eq!(positions(terminal.backend().buffer()), outside);
        }
        handle_key(
            &mut view,
            &snapshot,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        )
        .unwrap();
        terminal
            .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
            .unwrap();
        assert_eq!(positions(terminal.backend().buffer()), before);
    }
}
