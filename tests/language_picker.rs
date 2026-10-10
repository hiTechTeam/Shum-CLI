use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};
use ratatui_image::picker::Picker;
use serde_json::{json, Value};
use shum_cli::{
    display::Display,
    i18n,
    ui::{draw, handle_key, Action, Pictures, View},
};
const PEER: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
fn snapshot() -> Value {
    json!({"profile":{"ownerId":"own"},"card":{"name":"Me"}, "contacts":[
        {"id":PEER,"phase":"accepted","card":{"name":"Peer","avatarSeed":123}},
        {"id":"other","phase":"accepted","card":{"name":"Other","avatarSeed":42}}
    ],"messages": (0..15).map(|i| json!({"id":i.to_string(),"contactID":PEER,"text":"Hello"})).collect::<Vec<_>>(),
    "reactions":(0..15).map(|i|json!({"messageID":i.to_string(),"personID":PEER,"mark":{"reaction":"heart"}})).collect::<Vec<_>>()})
}
fn press(view: &mut View, code: KeyCode) -> Action {
    handle_key(view, &snapshot(), KeyEvent::new(code, KeyModifiers::NONE)).unwrap()
}
fn open(view: &mut View, letter: char) {
    handle_key(
        view,
        &snapshot(),
        KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL),
    )
    .unwrap();
}
#[test]
fn language_menu_selects_every_code_and_keeps_direct_commands_and_drafts() {
    for letter in ['l', 'д'] {
        let mut view = View::chat(PEER);
        view.input = "Unsent {draft}".into();
        open(&mut view, letter);
        for (index, code) in std::iter::once("system")
            .chain(i18n::LANGUAGES.iter().map(|(c, _)| *c))
            .enumerate()
        {
            press(&mut view, KeyCode::Home);
            for _ in 0..index {
                press(&mut view, KeyCode::Down);
            }
            assert!(matches!(press(&mut view,KeyCode::Enter),Action::Language(c) if c==code));
        }
        press(&mut view, KeyCode::Char('x'));
        press(&mut view, KeyCode::Esc);
        assert_eq!(view.input, "Unsent {draft}");
        view.input = "/language en".into();
        assert!(matches!(press(&mut view, KeyCode::Enter),Action::Language(c) if c=="en"));
        view.input = "/language".into();
        press(&mut view, KeyCode::Enter);
        assert!(view.input.is_empty());
        press(&mut view, KeyCode::End);
        assert!(matches!(press(&mut view,KeyCode::Enter),Action::Language(c) if c=="ko"));
        assert!(matches!(
            handle_key(
                &mut view,
                &snapshot(),
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)
            )
            .unwrap(),
            Action::None
        ));
        assert!(matches!(
            handle_key(
                &mut view,
                &snapshot(),
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)
            )
            .unwrap(),
            Action::Quit
        ));
    }
    let mut no_chat = View::default();
    open(&mut no_chat, 'l');
    press(&mut no_chat, KeyCode::Home);
    assert!(matches!(press(&mut no_chat,KeyCode::Enter),Action::Language(c) if c=="system"));
}
fn placements(buffer: &Buffer) -> Vec<Rect> {
    buffer
        .content
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if !c.symbol().contains("\x1b_Ga=T,") {
                return None;
            }
            let dimension = |key: &str| {
                c.symbol()
                    .split_once(key)
                    .unwrap()
                    .1
                    .split(',')
                    .next()
                    .unwrap()
                    .parse::<u16>()
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
#[test]
fn language_menu_only_covers_intersecting_pngs_and_restores_them_on_escape() {
    for (width, height) in [(160, 50), (80, 24), (35, 12)] {
        let display = Display::for_terminal("WarpTerminal", "xterm-256color", "", false, false);
        let mut picker = Picker::halfblocks();
        picker.set_protocol_type(display.images);
        let mut pictures = Pictures::with_display(picker, display);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut view = View::chat(PEER);
        let snapshot = snapshot();
        terminal
            .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
            .unwrap();
        let before = placements(terminal.backend().buffer());
        let w = 46.min(width);
        let h = 17.min(height);
        let popup = Rect::new((width - w) / 2, (height - h) / 2, w, h);
        let outside: Vec<_> = before
            .iter()
            .copied()
            .filter(|r| r.intersection(popup).is_empty())
            .collect();
        if width >= 80 {
            assert!(!outside.is_empty());
        }
        if width == 80 {
            assert!(outside.len() < before.len());
        }
        open(&mut view, 'l');
        for code in [KeyCode::Home, KeyCode::Down, KeyCode::End, KeyCode::Up] {
            press(&mut view, code);
            terminal
                .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
                .unwrap();
            assert_eq!(
                placements(terminal.backend().buffer()),
                outside,
                "{width}x{height}"
            );
            let text: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| {
                    if c.symbol().contains('\x1b') {
                        " "
                    } else {
                        c.symbol()
                    }
                })
                .collect();
            assert!(text.contains("Язык интерфейса"));
        }
        press(&mut view, KeyCode::Esc);
        terminal
            .draw(|f| draw(f, &snapshot, &mut view, &mut pictures, false))
            .unwrap();
        assert_eq!(placements(terminal.backend().buffer()), before);
    }
}
