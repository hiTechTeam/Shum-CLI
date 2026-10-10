use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};
use ratatui_image::picker::Picker;
use serde_json::{json, Value};
use shum_cli::{
    display::Display,
    ui::{draw, Pictures, View},
};

fn snapshot(a: Value, b: Value) -> Value {
    json!({"profile":{"ownerId":"own"}, "card":{"name":"Игорь"},
        "contacts":[{"id":"peer","phase":"accepted","card":{"name":"Аня"}}],
        "messages":[{"id":"one","contactID":"peer","text":"Сообщение","outgoing":false}],
        "reactions":[{"messageID":"one","personID":"peer","mark":{"reaction":a}},
                     {"messageID":"one","personID":"own","mark":{"reaction":b}}]})
}
fn render(snapshot: &Value, view: &mut View, warp: bool, width: u16, height: u16) -> Buffer {
    let display = Display::for_terminal(
        if warp {
            "WarpTerminal"
        } else {
            "Apple_Terminal"
        },
        "xterm-256color",
        "",
        false,
        false,
    );
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(display.images);
    let mut pictures = Pictures::with_display(picker, display);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| draw(f, snapshot, view, &mut pictures, false))
        .unwrap();
    terminal.backend().buffer().clone()
}
fn body(buffer: &Buffer) -> String {
    buffer
        .content
        .iter()
        .map(|c| {
            if c.symbol().contains('\x1b') {
                " "
            } else {
                c.symbol()
            }
        })
        .collect()
}
fn png_count(buffer: &Buffer) -> usize {
    buffer
        .content
        .iter()
        .filter(|c| c.symbol().contains("\x1b_Ga=T,"))
        .count()
}
#[test]
fn same_reaction_has_one_icon_and_both_names() {
    for warp in [true, false] {
        let buffer = render(
            &snapshot(json!("heart"), json!("heart")),
            &mut View::chat("peer"),
            warp,
            140,
            32,
        );
        let text = body(&buffer);
        assert!(text.contains("Аня / Игорь"));
        assert!(!text.contains("heart"));
        if warp {
            assert_eq!(png_count(&buffer), 1);
        } else {
            assert!(buffer
                .content
                .iter()
                .any(|c| matches!(c.bg, Color::Indexed(i) if i > 16 && i < 232)));
        }
    }
}
#[test]
fn distinct_reactions_have_two_icons_and_named_authors() {
    let buffer = render(
        &snapshot(json!("heart"), json!("fire")),
        &mut View::chat("peer"),
        true,
        140,
        32,
    );
    assert_eq!(png_count(&buffer), 2);
    let text = body(&buffer);
    assert!(text.contains("Аня / "));
    assert!(text.contains("Игорь"));
    assert!(!text.contains("heart") && !text.contains("fire"));
}
#[test]
fn removed_and_foreign_reactions_do_not_show_or_steal_names() {
    let mut s = snapshot(Value::Null, json!("like"));
    s["reactions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"messageID":"one","personID":"stranger","mark":{"reaction":"fire"}}));
    let buffer = render(&s, &mut View::chat("peer"), true, 140, 32);
    assert_eq!(png_count(&buffer), 1);
    assert!(body(&buffer).contains("Игорь"));
    assert!(!body(&buffer).contains("Аня /"));
    s["reactions"] = json!([]);
    assert_eq!(
        png_count(&render(&s, &mut View::chat("peer"), true, 140, 32)),
        0
    );
}
#[test]
fn equal_names_are_not_deduplicated() {
    let mut s = snapshot(json!("heart"), json!("heart"));
    s["card"]["name"] = json!("Аня");
    assert!(body(&render(&s, &mut View::chat("peer"), true, 140, 32)).contains("Аня / Аня"));
}
#[test]
fn icons_stay_inside_history_on_scroll_resize_and_modal() {
    let mut s = snapshot(json!("heart"), json!("fire"));
    s["card"]["name"] = json!("Очень длинное имя с emoji 👩‍💻");
    s["messages"][0]["outgoing"] = json!(true);
    s["messages"][0]["text"] = json!("Длинное сообщение ".repeat(30));
    for warp in [true, false] {
        for width in [35, 60, 100, 140] {
            for scroll in 0..20 {
                let mut view = View::chat("peer");
                view.scroll = scroll;
                let buffer = render(&s, &mut view, warp, width, 24);
                for (index, cell) in buffer.content.iter().enumerate() {
                    if cell.symbol().contains("\x1b_Ga=T,") {
                        assert!(cell.symbol().contains("c=4,r=2,"));
                        let row = index / usize::from(width);
                        assert!(row > 3 && row < 18, "reaction must stay above input: {row}");
                    }
                }
            }
        }
    }
    let mut view = View::chat("peer");
    view.help = true;
    assert_eq!(png_count(&render(&s, &mut view, true, 140, 32)), 0);
}
