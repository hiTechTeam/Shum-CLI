//! Client-side localization. Protocol values and user-authored text never enter this module.
use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        OnceLock,
    },
};

pub const LANGUAGES: [(&str, &str); 10] = [
    ("ru", "Русский"),
    ("en", "English"),
    ("es", "Español"),
    ("zh-Hans", "简体中文"),
    ("hi", "हिन्दी"),
    ("fr", "Français"),
    ("ja", "日本語"),
    ("pt-BR", "Português (Brasil)"),
    ("ar", "العربية"),
    ("ko", "한국어"),
];
// Library callers opt in; the executable initializes this before parsing localized help.
static SYSTEM: AtomicBool = AtomicBool::new(false);
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);
static CATALOGS: OnceLock<Vec<BTreeMap<String, String>>> = OnceLock::new();
fn catalogs() -> &'static [BTreeMap<String, String>] {
    CATALOGS.get_or_init(|| {
        [
            include_str!("../locales/ru.json"),
            include_str!("../locales/en.json"),
            include_str!("../locales/es.json"),
            include_str!("../locales/zh-Hans.json"),
            include_str!("../locales/hi.json"),
            include_str!("../locales/fr.json"),
            include_str!("../locales/ja.json"),
            include_str!("../locales/pt-BR.json"),
            include_str!("../locales/ar.json"),
            include_str!("../locales/ko.json"),
        ]
        .iter()
        .map(|s| serde_json::from_str(s).expect("validated embedded translation catalog"))
        .collect()
    })
}
pub fn t(source: &str) -> &str {
    translate(current(), source)
}
pub fn translate<'a>(language: &str, source: &'a str) -> &'a str {
    let index = LANGUAGES
        .iter()
        .position(|(code, _)| *code == language)
        .unwrap_or(1);
    catalogs()[index]
        .get(source)
        .or_else(|| catalogs()[1].get(source))
        .map(String::as_str)
        .unwrap_or(source)
}
pub fn current() -> &'static str {
    LANGUAGES[LANGUAGE.load(Ordering::Relaxed)].0
}
pub fn select(code: &str) -> Result<()> {
    let Some(index) = LANGUAGES.iter().position(|(c, _)| *c == code) else {
        bail!(
            "Unknown language: {code}. Use system, ru, en, es, zh-Hans, hi, fr, ja, pt-BR, ar, ko"
        );
    };
    LANGUAGE.store(index, Ordering::Relaxed);
    SYSTEM.store(false, Ordering::Relaxed);
    Ok(())
}
/// Values are interpolated once; braces in names/messages are always literal.
pub fn format(source: &str, values: &[(&str, String)]) -> String {
    interpolate(t(source), values)
}
pub fn interpolate(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    let mut ordinal = 0;
    while !rest.is_empty() {
        if rest.starts_with("{{") || rest.starts_with("}}") {
            out.push(rest.chars().next().unwrap());
            rest = &rest[2..];
            continue;
        }
        if let Some(end) = rest
            .strip_prefix('{')
            .and_then(|s| s.find('}').map(|n| n + 1))
        {
            let field = rest[1..end].split(':').next().unwrap_or("");
            let key = if field.is_empty() {
                let key = ordinal.to_string();
                ordinal += 1;
                key
            } else {
                field.into()
            };
            if let Some((_, value)) = values.iter().find(|(name, _)| *name == key) {
                out.push_str(value);
            } else {
                out.push_str(&rest[..=end]);
            }
            rest = &rest[end + 1..];
        } else {
            let c = rest.chars().next().unwrap();
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}
pub fn normalize(locale: &str) -> Option<&'static str> {
    let locale = locale
        .split(['.', '@'])
        .next()?
        .replace('_', "-")
        .to_lowercase();
    match locale.split('-').next()? {
        "ru" => Some("ru"),
        "en" | "c" | "posix" => Some("en"),
        "es" => Some("es"),
        "zh" => Some("zh-Hans"),
        "hi" => Some("hi"),
        "fr" => Some("fr"),
        "ja" => Some("ja"),
        "pt" => Some("pt-BR"),
        "ar" => Some("ar"),
        "ko" => Some("ko"),
        _ => None,
    }
}
fn system() -> &'static str {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return normalize(&value).unwrap_or("en");
            }
        }
    }
    #[cfg(target_os = "macos")]
    if let Ok(output) = std::process::Command::new("/usr/bin/defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
    {
        if output.status.success() {
            for word in String::from_utf8_lossy(&output.stdout)
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            {
                if let Some(code) = normalize(word) {
                    return code;
                }
            }
        }
    }
    "en"
}
pub fn settings_path(root: Option<&Path>) -> Result<PathBuf> {
    let root = match root {
        Some(root) => root.to_owned(),
        None => directories::ProjectDirs::from("org", "Shum", "Shum")
            .context("Cannot locate Shum settings")?
            .data_local_dir()
            .to_owned(),
    };
    Ok(root.join("cli-settings.json"))
}
pub fn configure(explicit: Option<&str>, root: Option<&Path>) -> Result<()> {
    let saved = std::fs::read(settings_path(root)?)
        .ok()
        .and_then(|data| serde_json::from_slice::<serde_json::Value>(&data).ok())
        .and_then(|v| v["language"].as_str().map(str::to_owned));
    let environment = std::env::var("SHUM_LANG").ok().filter(|s| !s.is_empty());
    let selected = explicit
        .or(environment.as_deref())
        .or(saved.as_deref())
        .unwrap_or("system");
    apply(selected)
}
pub fn save(code: &str, root: &Path) -> Result<()> {
    if code != "system" && !LANGUAGES.iter().any(|(c, _)| *c == code) {
        select(code)?;
    }
    std::fs::create_dir_all(root)?;
    let path = settings_path(Some(root))?;
    let mut data = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&bytes)
            .context("Invalid CLI settings")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(e) => return Err(e.into()),
    };
    data.insert("language".into(), code.into());
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec_pretty(&data)?)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    apply(code)
}
pub fn listing() -> String {
    let mut lines = vec![
        format!("{}: {}", t("Язык интерфейса"), current()),
        "system".into(),
    ];
    lines.extend(
        LANGUAGES
            .iter()
            .map(|(code, name)| std::format!("{code:8} {name}")),
    );
    lines.join("\n")
}

fn apply(code: &str) -> Result<()> {
    select(if code == "system" { system() } else { code })?;
    SYSTEM.store(code == "system", Ordering::Relaxed);
    Ok(())
}
pub fn preference() -> &'static str {
    if SYSTEM.load(Ordering::Relaxed) {
        "system"
    } else {
        current()
    }
}
