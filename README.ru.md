# Shum CLI

[English](README.md) · Русский

Клиент Shum для терминала на [Shum Core](https://github.com/hiTechTeam/Shum-Core). Чаты, приглашения, QR, профили, зашифрованная история и фоновая служба. Сообщения передаются по Bluetooth или через Nostr.

## Установка

Текущий выпуск: **0.1.7 preview, macOS 15+, Apple Silicon и Intel**.

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
shum contacts
shum add 'shum://c4/<signed-profile>'
shum add --image ~/Desktop/invite.png
shum invite 'CONTACT_ID'
shum open 'CONTACT_ID'
shum send 'CONTACT_ID' "Привет"
shum status
```

Замените CONTACT_ID на ID из `shum contacts`. Команды контактов принимают Shum ID, уникальный префикс от восьми символов или полный публичный Nostr-ключ. Имена служат для отображения. Полные карточки и QR добавляются офлайн; получение новой карточки по сетевому ID требует ответа собеседника.

В открытом чате `/invite` приглашает его собеседника. `/qr` показывает вашу карточку. `/help` открывает справку, Ctrl+Q закрывает интерфейс. В списке чатов видны непрочитанные сообщения и набор текста. Terminal рисует аватары цветом ячеек, Warp использует PNG.

После выхода служба продолжает работать. `shum daemon --stop` останавливает её; `shum daemon --install` включает запуск при входе. После обновления следующая команда CLI заменяет старую службу, сохраняя очередь. Разрешите доступ Shum к Bluetooth в запросе macOS.

Локальные профили выбираются по полному ID из `shum profile list`: `shum profile use 'PROFILE_ID'`, `shum -p 'PROFILE_ID' chats` или `shum profile delete 'PROFILE_ID'`. При удалении показываются имя и ID; ввод имени только подтверждает операцию, а не выбирает профиль.

Регистрация показывает анимацию сборки ключа, плавно заполняет шкалу и отмечает галочками создание ключей, шифрование, защищённое хранилище и проверку, затем спрашивает имя. После этого первый аватар вычисляется из открытого ключа подписи, как на iPhone. Ключи остаются в памяти до подтверждения аватара; для подготовки используется временное зашифрованное хранилище. Имена профилей могут совпадать, а ID, ключи и переписка независимы.

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

Данные остаются в `~/Library/Application Support/org.Shum.Shum` или вашем `--data-dir`; ключи могут храниться в Keychain.

Для безвозвратного удаления всех профилей, ключей (включая Keychain), переписки и кэша служб:

```sh
shum daemon --uninstall --purge
brew uninstall shum
```

Подтвердите удаление словом `DELETE`. Для скриптов добавьте `--confirm DELETE`. Доступно с версии 0.1.7. Без `--data-dir` учитываются стандартный каталог и другие каталоги данных, зарегистрированные в LaunchAgents Shum. С `--data-dir` удаляются только данные Shum в указанном каталоге. Посторонние файлы сохраняются, команда сообщает об этом. Установленное приложение отдельно удаляет пакетный менеджер или скрипт установки.

В версии 0.1.6 перед удалением служб и пакета удалите каждый профиль командой `shum profile delete 'PROFILE_ID'`. Она также удаляет ключи профиля из Keychain. После этого оставшийся каталог данных можно удалить вручную.

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

Версия 0.1.7 прошла 39 тестов CLI, проверку подписанного пакета, вывода в PTY и сценарий профилей и удаления под Rosetta. Замена службы 0.1.6 на 0.1.7 проверена на временном профиле. Сохранение разрешения Bluetooth и доставка на iPhone после этого обновления ещё требуют проверки на устройстве.

[Проверка выпуска](docs/release-verification-0.1.7.md) · [Распространение](docs/distribution.md) · [Состояние протокола](docs/protocol-status.md) · [Подробная инструкция](docs/cli-guide.ru.md) · [Issues](https://github.com/hiTechTeam/Shum-CLI/issues)

## Лицензия

[MIT](LICENSE), copyright 2026 hiTechTeam.
