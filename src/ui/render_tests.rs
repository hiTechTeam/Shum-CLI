use super::*;
use ratatui::{backend::TestBackend, Terminal};
use serde_json::json;

const PEER: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn fixture() -> Value {
    json!({"profile":{"ownerId":"me"},"card":{"name":"Me"},
        "contacts":[{"id":PEER,"phase":"accepted","card":{"name":"Peer","avatarSeed":42}}],
        "messages":[{"id":"one","contactID":PEER,"text":"Test message","outgoing":true,"timestamp":1000,"status":"queued"}],
        "reactions":[{"messageID":"one","personID":PEER,"mark":{"reaction":"heart"}}]})
}
fn pictures() -> Pictures {
    let display =
        crate::display::Display::for_terminal("WarpTerminal", "xterm-256color", "", false, false);
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(display.images);
    Pictures::with_display(picker, display)
}
fn mark(terminal: &mut Terminal<TestBackend>) {
    let mut cell = ratatui::buffer::Cell::default();
    cell.set_symbol("X");
    ratatui::backend::Backend::draw(terminal.backend_mut(), [(0, 0, &cell)].into_iter()).unwrap();
}
#[test]
fn delivery_updates_do_not_clear_warp_screen_or_move_wrapped_reactions() {
    let mut snapshot = fixture();
    let view = View::chat(PEER);
    let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
    let mut pictures = pictures();
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    mark(&mut terminal);
    for status in ["queued", "forwarding", "delivered", "read"] {
        snapshot["messages"][0]["status"] = json!(status);
        snapshot["messages"][0]["nostrAccepted"] = json!(true);
        snapshot["messages"][0]["pushScheduled"] = json!(true);
        pictures
            .clear_on_change(
                &mut terminal,
                graphics_scene("profile", &snapshot, &view, false),
            )
            .unwrap();
        assert_eq!(
            terminal.backend().buffer()[(0, 0)].symbol(),
            "X",
            "cleared on {status}"
        );
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(
                delivery_marker(&snapshot["messages"][0], false).as_str()
            ),
            2
        );
    }
    // Text reflow with existing reactions still invalidates old PNG placements.
    snapshot["messages"][0]["text"] = json!("long ".repeat(100));
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), " ");
}
#[test]
fn sending_without_history_images_keeps_screen_and_modals_still_invalidate() {
    let mut snapshot = fixture();
    snapshot["reactions"] = json!([]);
    let mut view = View::chat(PEER);
    let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
    let mut pictures = pictures();
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    mark(&mut terminal);
    snapshot["messages"].as_array_mut().unwrap().push(
        json!({"id":"new","contactID":PEER,"text":"New message","outgoing":true,"status":"queued"}),
    );
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "X");
    view.languages = Some(0);
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), " ");
    mark(&mut terminal);
    view.languages = None;
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("profile", &snapshot, &view, false),
        )
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), " ");
}

fn placements(buffer: &ratatui::buffer::Buffer) -> Vec<Rect> {
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
            let width = dimension("c=");
            let height = dimension("r=");
            Some(Rect::new(
                i as u16 % buffer.area.width + 1 - width,
                i as u16 / buffer.area.width + 1 - height,
                width,
                height,
            ))
        })
        .collect()
}

#[test]
fn every_menu_preserves_uncovered_chat_images_and_restores_covered_images() {
    for (width, height) in [(160, 50), (100, 32), (80, 24)] {
        for menu in [
            "profiles",
            "help",
            "info",
            "qr",
            "reactions",
            "languages",
            "delete",
            "clear",
        ] {
            let snapshot = fixture();
            let mut view = View::chat(PEER);
            let mut pictures = pictures();
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
                .unwrap();
            let before = placements(terminal.backend().buffer());
            assert!(before.len() >= 2);
            let expected_cover = match menu {
                "profiles" => {
                    view.profiles = Some(vec![]);
                    centered(Rect::new(0, 0, width, height), 52, 9)
                }
                "help" => {
                    view.help = true;
                    centered(Rect::new(0, 0, width, height), 76, 26)
                }
                "info" => {
                    view.info = Some(("Info".into(), "Body".into()));
                    centered(Rect::new(0, 0, width, height), 76, 26)
                }
                "qr" => {
                    let link = "shum://c2/test";
                    view.qr = Some(link.into());
                    let code = crate::terminal::qr(link, false).unwrap();
                    centered(
                        Rect::new(0, 0, width, height),
                        code.lines().next().unwrap().chars().count() as u16 + 4,
                        code.lines().count() as u16 + 4,
                    )
                }
                "reactions" => {
                    handle_key(
                        &mut view,
                        &snapshot,
                        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
                    )
                    .unwrap();
                    centered(Rect::new(0, 0, width, height), 72, 22)
                }
                "languages" => {
                    view.languages = Some(0);
                    languages::popup_area(Rect::new(0, 0, width, height))
                }
                "delete" => {
                    view.form = Some(Form::DeleteProfile {
                        id: "me".into(),
                        name: "Me".into(),
                    });
                    centered(Rect::new(0, 0, width, height), 56, 7)
                }
                "clear" => {
                    view.form = Some(Form::ClearChat("peer".into()));
                    centered(Rect::new(0, 0, width, height), 56, 7)
                }
                _ => unreachable!(),
            };
            let expected: Vec<_> = before
                .iter()
                .copied()
                .filter(|r| r.intersection(expected_cover).is_empty())
                .collect();
            if width == 160 {
                assert!(
                    !expected.is_empty(),
                    "wide window must keep uncovered images: {menu}"
                );
            }
            for _ in 0..3 {
                pictures
                    .clear_on_change(&mut terminal, graphics_scene("me", &snapshot, &view, false))
                    .unwrap();
                terminal
                    .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
                    .unwrap();
                assert_eq!(
                    placements(terminal.backend().buffer()),
                    expected,
                    "{menu} at {width}x{height}"
                );
            }
            view = View::chat(PEER);
            pictures
                .clear_on_change(&mut terminal, graphics_scene("me", &snapshot, &view, false))
                .unwrap();
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
                .unwrap();
            assert_eq!(
                placements(terminal.backend().buffer()),
                before,
                "closing {menu}"
            );
        }
    }
}

#[test]
fn nested_profile_confirmation_only_covers_overlapping_thumbnails() {
    let mut snapshot = fixture();
    snapshot["profile"]["id"] = json!("first");
    snapshot["card"]["avatarSeed"] = json!(42);
    let mut view = View::chat(PEER);
    view.profiles = Some(
        serde_json::from_value(json!([
            {"id":"first","name":"First","ownerId":"a","keyBackend":"file"},
            {"id":"second","name":"Second","ownerId":"b","keyBackend":"file"}
        ]))
        .unwrap(),
    );
    view.profile_details
        .insert("second".into(), json!({"card":{"avatarSeed":123}}));
    let mut pictures = pictures();
    let mut terminal = Terminal::new(TestBackend::new(160, 50)).unwrap();
    terminal
        .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    let before = placements(terminal.backend().buffer());
    assert_eq!(
        before
            .iter()
            .filter(|r| r.x == 56 && r.width == 6 && r.height == 3)
            .count(),
        2
    );
    view.form = Some(Form::DeleteProfile {
        id: "second".into(),
        name: "Second".into(),
    });
    let confirmation = Rect::new(52, 21, 56, 7);
    let expected: Vec<_> = before
        .iter()
        .copied()
        .filter(|r| r.intersection(confirmation).is_empty())
        .collect();
    assert_eq!(
        expected
            .iter()
            .filter(|r| r.x == 56 && r.width == 6 && r.height == 3)
            .count(),
        1
    );
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("first", &snapshot, &view, false),
        )
        .unwrap();
    terminal
        .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    assert_eq!(placements(terminal.backend().buffer()), expected);
    view.form = None;
    pictures
        .clear_on_change(
            &mut terminal,
            graphics_scene("first", &snapshot, &view, false),
        )
        .unwrap();
    terminal
        .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
        .unwrap();
    assert_eq!(placements(terminal.backend().buffer()), before);
}
