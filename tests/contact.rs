use serde_json::json;
use shum_cli::{contact, terminal};

#[test]
fn only_ids_select_recipients_and_exact_id_beats_prefix_or_network_alias() {
    let a = "a".repeat(64);
    let b = format!("{}{}", "a".repeat(8), "b".repeat(56));
    let network = "c".repeat(64);
    let candidates = [(a.as_str(), network.as_str()), (b.as_str(), a.as_str())];
    assert_eq!(
        contact::resolve(&a, candidates).unwrap().as_deref(),
        Some(a.as_str())
    );
    assert_eq!(
        contact::resolve(&network, candidates).unwrap().as_deref(),
        Some(a.as_str())
    );
    assert!(contact::resolve(&a[..8], candidates).is_err());
    assert_eq!(
        contact::resolve(&b[..9], candidates).unwrap().as_deref(),
        Some(b.as_str())
    );
    assert_eq!(contact::resolve(&"d".repeat(64), candidates).unwrap(), None);
    for name in ["Alice", "Аня", "abc", "ABCDEFGH", "0123456g", ""] {
        assert!(contact::resolve(name, candidates).is_err(), "{name}");
    }
    let snapshot = json!({"contacts":[
        {"id":a,"card":{"name":"Одинаковое имя","nostrKey":network}},
        {"id":b,"card":{"name":"Одинаковое имя","nostrKey":a}}
    ]});
    assert!(terminal::find_contact(&snapshot, "Одинаковое имя").is_err());
    assert_eq!(terminal::find_contact(&snapshot, &a).unwrap()["id"], a);
    assert_eq!(
        terminal::find_contact(&snapshot, &network).unwrap()["id"],
        a
    );
}
