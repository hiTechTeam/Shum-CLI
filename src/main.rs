#![forbid(unsafe_code)]
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use shum_cli::i18n::{self, t};
use shum_cli::{
    ipc, onboarding,
    runtime::{self, Request},
    terminal::{self, Palette, Tone},
    ui,
};
use shum_core::{card::Card, crypto, packet::ReactionKind};
use shum_store::{profiles::Profiles, vault::KeyMode};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
};
const HELP_STYLES: clap::builder::Styles = clap::builder::Styles::styled()
    .header(clap::builder::styling::AnsiColor::Green.on_default().bold())
    .usage(clap::builder::styling::AnsiColor::Green.on_default().bold())
    .literal(clap::builder::styling::AnsiColor::Cyan.on_default())
    .placeholder(clap::builder::styling::AnsiColor::Cyan.on_default());
#[derive(Parser)]
#[command(
    name = "shum",
    version,
    styles = HELP_STYLES,
    about = t("Мессенджер без номера телефона"),
    after_help = t("Без команды открываются чаты. При первом запуске Shum предложит создать профиль.")
)]
struct Args {
    #[arg(help = t("Язык интерфейса"))]
    #[arg(long, global = true)]
    lang: Option<String>,
    #[arg(short = 'p', long, global = true)]
    #[arg(help = t("ID локального профиля из shum profile list"))]
    profile: Option<String>,
    #[arg(long, global = true)]
    #[arg(help = t("Вывод JSON для скриптов"))]
    json: bool,
    #[arg(long, global = true)]
    #[arg(help = t("Без цвета и аватаров"))]
    ascii: bool,
    #[arg(long, global = true, env = "SHUM_DATA_DIR")]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    #[arg(help = t("Адрес релея; можно повторить для нескольких"))]
    relay: Vec<String>,
    #[arg(long, global = true)]
    #[arg(help = t("Адрес push API или off"))]
    push_url: Option<String>,
    #[arg(long, global = true, conflicts_with = "no_bluetooth")]
    #[arg(help = t("Включить Bluetooth для выбранного профиля"))]
    bluetooth: bool,
    #[arg(long, global = true)]
    #[arg(help = t("Работать только через интернет"))]
    no_bluetooth: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    #[command(about = t("Показать или сохранить язык интерфейса"))]
    Language { code: Option<String> },
    #[command(about = t("Создать профиль и ключи"))]
    Init {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        headless: bool,
    },
    #[command(about = t("Профиль, имя и аватар"))]
    Profile {
        #[command(subcommand)]
        command: Option<ProfileCommand>,
    },
    #[command(about = t("Пригласить по Shum ID или полному сетевому ID"))]
    Invite {
        #[arg(value_name = "ID")]
        contact: String,
    },
    #[command(about = t("Мой QR и полная карточка контакта"))]
    Qr,
    #[command(about = t("Добавить контакт по ссылке или QR"))]
    Add {
        link: Option<String>,
        #[arg(long)]
        image: Option<PathBuf>,
    },
    #[command(about = t("Список контактов и их Shum ID"))]
    Contacts,
    #[command(about = t("Принять приглашение в чат"))]
    Accept {
        #[arg(value_name = "ID")]
        contact: String,
    },
    #[command(about = t("Отклонить приглашение"))]
    Decline {
        #[arg(value_name = "ID")]
        contact: String,
    },
    #[command(about = t("Чаты и фильтры"))]
    Chats {
        #[arg(long)]
        nearby: bool,
        #[arg(long)]
        invites: bool,
        #[arg(long)]
        unread: bool,
    },
    #[command(about = t("Открыть чат; вне терминала ждать новые сообщения"))]
    Open {
        #[arg(value_name = "ID")]
        contact: String,
    },
    #[command(about = t("Терминальный интерфейс, при желании сразу в чате"))]
    Ui {
        #[arg(value_name = "ID")]
        contact: Option<String>,
    },
    #[command(about = t("Отправить сообщение принятому контакту"))]
    Send {
        #[arg(value_name = "ID")]
        contact: String,
        text: String,
    },
    #[command(about = t("Отметить чат прочитанным"))]
    Read {
        #[arg(value_name = "ID")]
        contact: String,
    },
    #[command(about = t("Поставить или снять реакцию на сообщение"))]
    React { message: String, reaction: String },
    #[command(about = t("Очистить чат на этом компьютере"))]
    Clear {
        #[arg(value_name = "ID")]
        contact: String,
        #[arg(long)]
        confirm: bool,
    },
    #[command(about = t("Отменить отправку сообщения"))]
    Cancel { message: String },
    #[command(about = t("Заблокировать контакт; --undo разблокировать"))]
    Block {
        #[arg(value_name = "ID")]
        contact: String,
        #[arg(long)]
        undo: bool,
    },
    #[command(about = t("Сверить отпечатки ключей"))]
    Keys {
        #[command(subcommand)]
        command: KeysCommand,
    },
    #[command(about = t("Остановить и заблокировать сеанс профиля"))]
    Lock,
    #[command(about = t("Снять блокировку сеанса"))]
    Unlock,
    #[command(about = t("Подключения и состояние службы"))]
    Status,
    #[command(about = t("Версия, профиль и каталог данных"))]
    About,
    #[command(about = t("Устройства рядом и состояние Bluetooth"))]
    Nearby,
    #[command(about = t("Фоновая служба: автозапуск или остановка"))]
    Daemon {
        #[arg(long, hide = true, conflicts_with_all = ["install", "stop", "uninstall", "refresh"])]
        run: bool,
        #[arg(long, conflicts_with_all = ["stop", "uninstall", "refresh"])]
        install: bool,
        #[arg(long, conflicts_with_all = ["uninstall", "refresh"])]
        stop: bool,
        #[arg(help = t("Остановить все службы и удалить автозапуск, сохранив данные"))]
        #[arg(long)]
        uninstall: bool,
        #[arg(help = t("Также безвозвратно удалить все профили, ключи и историю"))]
        #[arg(long, requires = "uninstall")]
        purge: bool,
        #[arg(help = t("Подтверждение полного удаления: DELETE"))]
        #[arg(long, requires = "purge")]
        confirm: Option<String>,
        /// Used by the standalone installer to update active profiles only
        #[arg(long, hide = true, conflicts_with = "uninstall")]
        refresh: bool,
    },
}
#[derive(Subcommand)]
enum ProfileCommand {
    List,
    Use {
        #[arg(value_name = "PROFILE_ID")]
        id: String,
    },
    Delete {
        #[arg(value_name = "PROFILE_ID")]
        id: String,
        #[arg(long)]
        confirm: Option<String>,
    },
    Name {
        name: String,
    },
    Bio {
        text: String,
    },
    Avatar {
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        random: bool,
        #[arg(long)]
        photo: Option<PathBuf>,
    },
}
#[derive(Subcommand)]
enum KeysCommand {
    Verify {
        #[arg(value_name = "ID")]
        contact: String,
        #[arg(long)]
        qr: bool,
    },
}
fn output(value: Value, json_mode: bool, plain: &str) -> Result<()> {
    if json_mode {
        println!("{}", serde_json::to_string(&value)?);
    } else {
        println!("{plain}");
    }
    Ok(())
}
fn root(args: &Args) -> Result<PathBuf> {
    if let Some(path) = &args.data_dir {
        return Ok(std::path::absolute(path)?);
    }
    Ok(directories::ProjectDirs::from("org", "Shum", "Shum")
        .context(t("Укажите --data-dir"))?
        .data_local_dir()
        .to_owned())
}
#[tokio::main]
async fn main() -> std::process::ExitCode {
    let json_mode = std::env::args().any(|a| a == "--json");
    let ascii_mode = std::env::args().any(|a| a == "--ascii");
    let raw: Vec<String> = std::env::args().collect();
    let option = |name: &str| {
        raw.iter()
            .take_while(|a| a.as_str() != "--")
            .enumerate()
            .find_map(|(i, arg)| {
                if arg == name {
                    raw.get(i + 1).cloned()
                } else {
                    arg.strip_prefix(&format!("{name}=")).map(str::to_owned)
                }
            })
    };
    let language_root = option("--data-dir")
        .or_else(|| std::env::var("SHUM_DATA_DIR").ok())
        .map(PathBuf::from);
    if let Err(error) = i18n::configure(option("--lang").as_deref(), language_root.as_deref()) {
        if json_mode {
            println!("{}", json!({"error":error.to_string()}));
        } else {
            eprintln!("Shum: {error}");
        }
        return std::process::ExitCode::from(2);
    }
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let help = matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            );
            if json_mode {
                println!("{}", json!({"message":error.to_string(),"success":help}));
            } else if ascii_mode {
                if help {
                    print!("{error}");
                } else {
                    eprint!("{error}");
                }
            } else {
                let _ = error.print();
            }
            return std::process::ExitCode::from(if help { 0 } else { 2 });
        }
    };
    match run(args).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if json_mode {
                println!("{}", json!({"error":error.to_string()}));
            } else {
                eprintln!(
                    "{}",
                    Palette::stderr(ascii_mode).paint(Tone::Error, format!("Shum: {error}"))
                );
            }
            std::process::ExitCode::from(1)
        }
    }
}
async fn run(args: Args) -> Result<()> {
    // Capture our executable before Homebrew can replace this installation.
    let _build = shum_cli::identity::current()?;
    let palette = Palette::stdout(args.ascii);
    let root = root(&args)?;
    if let Some(Commands::Language { code }) = &args.command {
        if let Some(code) = code {
            i18n::save(code, &root)?;
        }
        if args.json {
            println!(
                "{}",
                json!({"language":i18n::current(),"languages":i18n::LANGUAGES})
            );
        } else {
            println!("{}", i18n::listing());
        }
        return Ok(());
    }
    if let Some(Commands::Daemon {
        uninstall: true,
        purge,
        confirm,
        ..
    }) = &args.command
    {
        let result = if *purge {
            let typed = match confirm {
                Some(value) => value.clone(),
                None if io::stdin().is_terminal() && !args.json => terminal::prompt(t(
                    "Все профили, ключи и переписка будут удалены безвозвратно. Введите DELETE: ",
                ))?,
                _ => bail!("{}", t("Для полного удаления укажите --confirm DELETE")),
            };
            if typed != "DELETE" {
                bail!("{}", t("Подтверждение не совпало, удаление отменено"));
            }
            shum_cli::service::purge(&root, args.data_dir.is_none()).await?
        } else {
            shum_cli::service::uninstall(&root, args.data_dir.is_none()).await?
        };
        return output(
            result,
            args.json,
            &palette.paint(
                Tone::Accent,
                if *purge {
                    t("Службы, профили, ключи и переписка удалены. Пакет Homebrew: brew uninstall shum.")
                } else {
                    t("Службы и автозапуск удалены. Данные профилей сохранены.")
                },
            ),
        );
    }
    if matches!(args.command, Some(Commands::Daemon { refresh: true, .. })) {
        let result = shum_cli::service::refresh(&root, args.data_dir.is_none()).await?;
        return output(result, args.json, t("Активные службы обновлены."));
    }
    let profiles = Profiles::new(&root)?;
    let settings = onboarding::Settings {
        bluetooth: args.bluetooth
            || (!args.no_bluetooth
                && !matches!(&args.command, Some(Commands::Init { headless: true, .. }))),
        relays: if args.relay.is_empty() {
            onboarding::Settings::default().relays
        } else {
            args.relay.clone()
        },
        push_url: match args.push_url.as_deref() {
            Some("off") => None,
            Some(url) => Some(url.into()),
            None => onboarding::Settings::default().push_url,
        },
    };
    if let Some(Commands::Init { name, headless }) = &args.command {
        let mode = if *headless {
            KeyMode::File
        } else {
            KeyMode::Auto
        };
        let name = name.as_ref();
        let created = if !headless
            && io::stdin().is_terminal()
            && io::stdout().is_terminal()
            && !args.json
        {
            let Some(created) =
                onboarding::run_named(&root, args.ascii, mode, settings, name.map(String::as_str))
                    .await?
            else {
                return Ok(());
            };
            created
        } else if let Some(name) = name {
            onboarding::create(
                &root,
                name,
                None,
                mode,
                &settings,
                shum_store::vault::ProfileKeys::generate()?,
            )?
        } else if io::stdin().is_terminal() && io::stdout().is_terminal() && !args.json {
            let Some(created) = onboarding::run(&root, args.ascii, mode, settings).await? else {
                return Ok(());
            };
            created
        } else {
            bail!("{}", t("Укажите shum init --name <имя>"));
        };
        if !args.json {
            terminal::avatar(created.card.avatar_seed.unwrap_or(0), args.ascii);
        }
        return output(
            onboarding::json(&created)?,
            args.json,
            &palette.paint(Tone::Accent, onboarding::completion(&created)),
        );
    }
    if matches!(args.command, None | Some(Commands::Ui { .. }))
        && profiles.list()?.1.is_empty()
        && io::stdin().is_terminal()
        && io::stdout().is_terminal()
        && !args.json
        && onboarding::run(&root, args.ascii, KeyMode::Auto, settings)
            .await?
            .is_none()
    {
        return Ok(());
    }
    if let Some(Commands::Profile {
        command: Some(ProfileCommand::List),
    }) = &args.command
    {
        let (selected, list) = profiles.list()?;
        if args.json {
            return output(json!({"selected":selected,"profiles":list}), true, "");
        }
        for p in list {
            if !args.ascii {
                if let Some(detail) = ui::profile_preview(&root, &p.id).await {
                    if let Some(seed) = detail["card"]["avatarSeed"].as_u64() {
                        terminal::avatar(seed, false);
                    }
                }
            }
            println!(
                "{} {}",
                if selected.as_deref() == Some(&p.id) {
                    ">"
                } else {
                    " "
                },
                palette.paint(
                    if selected.as_deref() == Some(&p.id) {
                        Tone::Accent
                    } else {
                        Tone::Text
                    },
                    terminal::safe(&p.name)
                )
            );
            println!("  ID: {}", p.id);
        }
        return Ok(());
    }
    if let Some(Commands::Profile {
        command: Some(ProfileCommand::Use { id }),
    }) = &args.command
    {
        let profile = ipc::select(&profiles, Some(id))?;
        profiles.select(id)?;
        return output(
            json!({"selected":id}),
            args.json,
            &palette.paint(
                Tone::Accent,
                i18n::format(
                    "Выбран профиль {} (ID {})",
                    &[
                        ("0", terminal::safe(&profile.name).to_string()),
                        ("1", id.to_string()),
                    ],
                ),
            ),
        );
    }
    if let Some(Commands::Profile {
        command: Some(ProfileCommand::Delete { id, confirm }),
    }) = &args.command
    {
        let profile = profiles
            .list()?
            .1
            .into_iter()
            .find(|profile| &profile.id == id)
            .context(t("Укажите полный ID профиля из shum profile list; удаление по имени не поддерживается"))?;
        let typed = match confirm {
            Some(name) => name.clone(),
            None if io::stdin().is_terminal() && !args.json => terminal::prompt(&i18n::format(
                "Удалить профиль {} (ID {}) и историю? Введите имя для подтверждения: ",
                &[
                    ("0", terminal::safe(&profile.name).to_string()),
                    ("1", profile.id.to_string()),
                ],
            ))?,
            _ => bail!(
                "{}",
                i18n::format(
                    "Для удаления укажите --confirm {:?}",
                    &[("0", format!("{:?}", profile.name))]
                )
            ),
        };
        if typed != profile.name {
            bail!("{}", t("Имя не совпало, удаление отменено"));
        }
        ipc::stop(&root, &profile.id).await?;
        profiles.delete(&profile.id)?;
        return output(
            json!({"deleted":profile.id}),
            args.json,
            &palette.paint(Tone::Accent, t("Профиль удалён")),
        );
    }
    let profile = ipc::select(&profiles, args.profile.as_deref())?;
    if let Some(Commands::Daemon { run: true, .. }) = &args.command {
        return ipc::serve(&root, &profile.id).await;
    }
    if matches!(args.command, Some(Commands::Lock)) {
        ipc::stop(&root, &profile.id).await?;
        let mut file = tempfile::NamedTempFile::new_in(root.join(&profile.id))?;
        file.write_all(b"locked\n")?;
        file.persist(root.join(&profile.id).join("locked"))?;
        return output(
            json!({"locked":true}),
            args.json,
            &palette.paint(
                Tone::Accent,
                t("Профиль заблокирован. Для продолжения: shum unlock"),
            ),
        );
    }
    if matches!(args.command, Some(Commands::Unlock)) {
        let open = profiles.open(Some(&profile.id))?;
        drop(open);
        let path = root.join(&profile.id).join("locked");
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        ipc::ensure(&root, &profile.id).await?;
        return output(
            json!({"locked":false}),
            args.json,
            &palette.paint(Tone::Accent, t("Профиль открыт")),
        );
    }
    if let Some(Commands::Daemon { stop: true, .. }) = &args.command {
        let _control = shum_cli::service::control_lock(&root).await?;
        ipc::stop(&root, &profile.id).await?;
        return output(
            json!({"stopped":true}),
            args.json,
            &palette.paint(Tone::Accent, t("Служба остановлена")),
        );
    }
    if let Some(Commands::Daemon { install: true, .. }) = &args.command {
        let _control = shum_cli::service::control_lock(&root).await?;
        ipc::stop(&root, &profile.id).await?;
        shum_cli::service::install(&root, &profile.id)?;
        return output(
            json!({"installed":true}),
            args.json,
            &palette.paint(Tone::Accent, t("Автозапуск службы установлен")),
        );
    }
    if !args.relay.is_empty() || args.push_url.is_some() || args.bluetooth || args.no_bluetooth {
        if !args.relay.is_empty() {
            shum_transport_nostr::RelayPool::validate_urls(&args.relay)?;
        }
        if let Some(url) = args.push_url.as_deref().filter(|url| *url != "off") {
            shum_transport_nostr::push::PushClient::new(url)?;
        }
        ipc::stop(&root, &profile.id).await?;
        let mut open = profiles.open(Some(&profile.id))?;
        open.store.transaction(|s| {
            if !s["cliSettings"].is_object() {
                s["cliSettings"] = json!({});
            }
            if args.bluetooth || args.no_bluetooth {
                s["cliSettings"]["bluetooth"] = json!(args.bluetooth);
            }
            if !args.relay.is_empty() {
                s["cliSettings"]["relays"] = json!(args.relay);
            }
            if let Some(url) = &args.push_url {
                s["cliSettings"]["pushURL"] = if url == "off" {
                    Value::Null
                } else {
                    json!(url)
                };
            }
            Ok(())
        })?;
        drop(open);
    }
    ipc::ensure(&root, &profile.id).await?;
    let request = match &args.command {
        None | Some(Commands::Ui { .. }) | Some(Commands::Open { .. }) => {
            let contact = match &args.command {
                Some(Commands::Ui { contact }) => contact.as_deref(),
                Some(Commands::Open { contact }) => Some(contact.as_str()),
                _ => None,
            };
            if io::stdout().is_terminal() && io::stdin().is_terminal() && !args.json {
                return ui::run(&root, &profile.id, contact, args.ascii).await;
            }
            if let Some(contact) = contact {
                return stream_chat(&root, &profile.id, contact, args.json).await;
            }
            let snapshot = ipc::request(&root, &profile.id, Request::Snapshot).await?;
            return output(
                snapshot,
                args.json,
                t("TUI требует терминал. Используйте shum chats или --json."),
            );
        }
        Some(Commands::Add { link, image }) => {
            if link.is_some() == image.is_some() {
                bail!("{}", t("Укажите одну ссылку либо --image <png>"));
            }
            Request::Add {
                link: if let Some(path) = image {
                    terminal::decode_qr(path)?
                } else {
                    link.clone().unwrap()
                },
            }
        }
        Some(Commands::Invite { contact }) => Request::Invite {
            contact: contact.clone(),
        },
        Some(Commands::Accept { contact }) => Request::Accept {
            contact: contact.clone(),
        },
        Some(Commands::Decline { contact }) => Request::Decline {
            contact: contact.clone(),
        },
        Some(Commands::Send { contact, text }) => Request::Send {
            contact: contact.clone(),
            text: text.clone(),
        },
        Some(Commands::Read { contact }) => Request::Read {
            contact: contact.clone(),
        },
        Some(Commands::React { message, reaction }) => Request::Reaction {
            message: message.clone(),
            reaction: serde_json::from_value::<ReactionKind>(json!(reaction))
                .context(t("Неизвестная реакция"))?,
        },
        Some(Commands::Clear { contact, confirm }) => {
            if !confirm
                && (!io::stdin().is_terminal()
                    || terminal::prompt(t("Очистить этот чат здесь? Напишите да: "))? != t("да"))
            {
                bail!("{}", t("Для очистки требуется --confirm"));
            }
            Request::Clear {
                contact: contact.clone(),
            }
        }
        Some(Commands::Cancel { message }) => Request::Cancel {
            message: message.clone(),
        },
        Some(Commands::Block { contact, undo }) => Request::Block {
            contact: contact.clone(),
            blocked: !*undo,
        },
        Some(Commands::Profile {
            command: Some(ProfileCommand::Name { name }),
        }) => Request::Profile {
            name: Some(name.clone()),
            bio: None,
            seed: None,
        },
        Some(Commands::Profile {
            command: Some(ProfileCommand::Bio { text }),
        }) => Request::Profile {
            name: None,
            bio: Some(text.clone()),
            seed: None,
        },
        Some(Commands::Profile {
            command:
                Some(ProfileCommand::Avatar {
                    seed,
                    random,
                    photo,
                }),
        }) => {
            if photo.is_some() {
                bail!(
                    "{}",
                    t("Фото-аватары пока недоступны. iPhone 1.0 не принимает пакеты фото.")
                );
            }
            let seed = if *random {
                Some(u64::from_le_bytes(runtime::random()?))
            } else {
                *seed
            };
            if seed.is_none() {
                bail!("{}", t("Укажите --random или --seed <число>"));
            }
            Request::Profile {
                name: None,
                bio: None,
                seed,
            }
        }
        _ => Request::Snapshot,
    };
    let value = ipc::request(&root, &profile.id, request).await?;
    match args.command {
        Some(Commands::Qr) => {
            let card: Card = serde_json::from_value(value["card"].clone())?;
            let link = card.invitation()?;
            if args.json {
                output(json!({"link":link,"errorCorrection":"M"}), true, "")
            } else {
                terminal::print_qr(&link, args.ascii)?;
                println!("{}", palette.paint(Tone::Command, &link));
                Ok(())
            }
        }
        Some(Commands::Profile { command: None }) => {
            if args.json {
                output(value, true, "")
            } else {
                terminal::profile(&value, args.ascii);
                Ok(())
            }
        }
        Some(Commands::Chats {
            nearby,
            invites,
            unread,
        }) => {
            if args.json {
                let mut filtered = value.clone();
                filtered["contacts"] = value["contacts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|c| {
                        (!nearby || c["nearby"] == true)
                            && (!invites || shum_cli::invitations::is_invitation(c))
                            && (!unread || c["unread"].as_u64().unwrap_or(0) > 0)
                    })
                    .cloned()
                    .collect();
                output(filtered, true, "")
            } else {
                terminal::chats(&value, nearby, invites, unread, args.ascii);
                Ok(())
            }
        }
        Some(Commands::Contacts) => {
            if args.json {
                output(value["contacts"].clone(), true, "")
            } else {
                for c in value["contacts"].as_array().into_iter().flatten() {
                    println!(
                        "{}  {}",
                        terminal::safe(terminal::text(&c["card"]["name"])),
                        palette.paint(Tone::Command, terminal::safe(terminal::text(&c["id"])))
                    );
                }
                Ok(())
            }
        }
        Some(Commands::Keys {
            command: KeysCommand::Verify { contact, qr },
        }) => {
            let peer = terminal::find_contact(&value, &contact)?;
            let card: Card = serde_json::from_value(peer["card"].clone())?;
            let fingerprint = crypto::id(&card.noise_key);
            let signing = crypto::id(&card.signing_key);
            if args.json {
                output(
                    json!({"iosFingerprint":terminal::fingerprint(&card),"noiseFingerprint":fingerprint,"signingFingerprint":signing,"card":card}),
                    true,
                    "",
                )
            } else {
                println!("{}", i18n::format("{}\nОтпечаток в iPhone: {}\nNoise / Shum ID: {}\nEd25519: {}\nСверьте отпечаток с собеседником: Профиль → Безопасность.", &[("0", terminal::safe(&card.name).to_string()), ("1", palette.paint(Tone::Accent, terminal::fingerprint(&card)).to_string()), ("2", palette.paint(Tone::Command, fingerprint).to_string()), ("3", palette.paint(Tone::Command, signing).to_string())]));
                if qr {
                    terminal::print_qr(&card.invitation()?, args.ascii)?;
                }
                Ok(())
            }
        }
        Some(Commands::Nearby) => {
            let peers = value["contacts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| p["nearby"] == true)
                .cloned()
                .collect::<Vec<_>>();
            if args.json {
                output(
                    json!({"bluetooth":value["bluetooth"],"peers":peers}),
                    true,
                    "",
                )
            } else {
                println!("{}", terminal::bluetooth(&value, palette));
                terminal::chats(&value, true, false, false, args.ascii);
                Ok(())
            }
        }
        Some(Commands::About) => {
            let description = terminal::about(&value, &root, args.ascii);
            output(value, args.json, &description)
        }
        Some(Commands::Status) | Some(Commands::Daemon { .. }) => {
            let description = terminal::status(&value, &root, args.ascii);
            output(value, args.json, &description)
        }
        _ => output(
            value,
            args.json,
            &palette.paint(Tone::Accent, t("Сохранено")),
        ),
    }
}
async fn stream_chat(
    root: &std::path::Path,
    profile: &str,
    contact: &str,
    json_mode: bool,
) -> Result<()> {
    let mut seen = std::collections::HashMap::new();
    loop {
        let snapshot = ipc::request(root, profile, Request::Snapshot).await?;
        let id = terminal::text(&terminal::find_contact(&snapshot, contact)?["id"]);
        for message in shum_cli::invitations::timeline(&snapshot, id) {
            if seen
                .insert(
                    message["id"].as_str().unwrap_or("").to_owned(),
                    message.clone(),
                )
                .as_ref()
                != Some(message)
            {
                if json_mode {
                    println!("{message}");
                } else if message["kind"].is_string() {
                    println!("{}", shum_cli::invitations::label(message));
                } else {
                    println!(
                        "{}: {}",
                        if message["outgoing"] == true {
                            t("я")
                        } else {
                            contact
                        },
                        terminal::safe(terminal::text(&message["text"]))
                    );
                }
            }
        }
        io::stdout().flush()?;
        ipc::request(
            root,
            profile,
            Request::Focus {
                contact: Some(id.into()),
            },
        )
        .await?;
        tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_secs(1))=>{}}
    }
    ipc::request(root, profile, Request::Focus { contact: None }).await?;
    Ok(())
}
