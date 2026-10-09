# Shum CLI

[English](README.md) · Русский

Клиент Shum для терминала на [Shum Core](https://github.com/hiTechTeam/Shum-Core). Чаты, приглашения, QR, профили, зашифрованная история и фоновая служба. Сообщения передаются по Bluetooth или через Nostr.

## Установка

Текущий выпуск: **0.1.6 preview, macOS 15+, Apple Silicon и Intel**.

```sh
brew install hitechteam/shum/shum
shum
```

Первый запуск предлагает создать профиль. Обновление:

```sh
brew update
brew upgrade shum
```

Без Homebrew установка и обновление выполняются одной командой:

```sh
curl -fsSL https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh | sh
```

Скрипт проверяет SHA-256 и подпись, устанавливает бандл в `~/.local/share/shum` и ссылку `~/.local/bin/shum`. При необходимости показывает подсказку PATH, файлы оболочки не меняет. Установка рядом с Homebrew отклоняется.

В [загрузках](https://github.com/hiTechTeam/Shum-CLI/releases) есть универсальный архив и установщик macOS. Сертификат самоподписанный, не Apple Developer ID; нотарификации нет. Gatekeeper на чистой учётной записи и настоящий Intel Mac пока не проверены.

## Использование

```sh
shum chats
shum profile
shum qr
shum add 'shum://c4/<signed-profile>'
shum add --image ~/Desktop/invite.png
shum invite <contact-id>
shum open <contact-id>
shum send <contact-id> "Привет"
shum status
```

Команды контактов принимают Shum ID, уникальный префикс от восьми символов или полный публичный Nostr-ключ. Имена служат для отображения. Полные карточки и QR добавляются офлайн; получение новой карточки по сетевому ID требует ответа собеседника.

В открытом чате `/invite` приглашает его собеседника. `/qr` показывает вашу карточку. `/help` открывает справку, Ctrl+Q закрывает интерфейс. В списке чатов видны непрочитанные сообщения и набор текста. Terminal рисует аватары цветом ячеек, Warp использует PNG.

После выхода служба продолжает работать. `shum daemon --stop` останавливает её; `shum daemon --install` включает запуск при входе. После обновления следующая команда CLI заменяет старую службу, сохраняя очередь. Разрешите доступ Shum к Bluetooth в запросе macOS.

## Удаление

Для Homebrew:

```sh
shum daemon --uninstall
brew uninstall shum
```

Для установки скриптом:

```sh
curl -fsSL https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh | sh -s -- --uninstall
```

Данные остаются в `~/Library/Application Support/org.Shum.Shum` или вашем `--data-dir`; ключи могут храниться в Keychain. Для полного стирания остановите службы, вручную удалите каталог данных и объекты Shum этого профиля в Keychain.

Если бинарник уже удалён, выгрузите каждый LaunchAgent Shum и удалите его plist:

```sh
launchctl bootout "gui/$(id -u)" "$HOME/Library/LaunchAgents/org.shum.cli.<profile-id>.plist"
rm "$HOME/Library/LaunchAgents/org.shum.cli.<profile-id>.plist"
```

Подставьте ID из имени файла. `KeepAlive=false` предотвращает цикл перезапуска без бинарника.

## Разработка и состояние

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

CLI и Core имеют отдельные репозитории и выпуски. Все зависимости ядра закреплены одной Git-ревизией. CI запускается вручную. Клиент реализует черновик v1; синхронизация устройств впереди. Пакеты Linux и Windows пока готовятся.

Выпуск 0.1.6 прошёл 31 тест CLI и проверку переписки под Rosetta. Проверены Homebrew-обновление, автоматическая замена службы и сообщения с iPhone в обе стороны. При переходе с ad hoc на сертификатную подпись Bluetooth запросил доступ заново; следующее обновление между сертификатными сборками ещё нужно проверить.

[Проверка выпуска](docs/release-verification.md) · [Распространение](docs/distribution.md) · [Состояние протокола](docs/protocol-status.md) · [Подробная инструкция](docs/cli-guide.ru.md) · [Issues](https://github.com/hiTechTeam/Shum-CLI/issues)

## Лицензия

[MIT](LICENSE), copyright 2026 hiTechTeam.
