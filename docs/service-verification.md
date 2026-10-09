# Проверка подписи и жизненного цикла службы

Дата: 9 октября 2026. Проверка выполнена на Mac владельца, Apple Silicon.
Первоначальные проверки выполнены до синхронизации исходников. Результаты
сохраняются здесь как отчёт этого этапа; публикация и обновление публичного tap
не выполнялись. Установленная формула и реальные профили не изменялись.
`tests/tui.rs` сохранён побайтно относительно состояния перед задачей:
SHA-256 `6e90abd725b58572c4807abca1ea3ec919feb3641cd346906fa14f43e3a8538e`.

## Сделано

* Сертификат Shum CLI Signing создан через «Связку ключей», в login.
  RSA 2048, срок до 6 октября 2036 года. Закрытый ключ не экспортировался;
  системные запросы codesign завершал владелец Mac. Системное доверие и TCC
  не изменялись.
* Версия встроенного и внешнего Info.plist берётся из Cargo.toml.
* Упаковка создаёт сертификатный Shum.app, архив с бандлом, установщик и
  проект формулы с homepage Shum-CLI, ссылкой bin/shum и одной строкой caveats.
  Установщик не перемещает бандл в найденный сторонний каталог: relocation
  отключён. Контейнер .pkg остаётся без Developer ID Installer и нотарификации.
* Runtime использует установленный бандл напрямую. LaunchAgent для Homebrew
  ссылается на opt/shum. Для Cargo сохраняется ad hoc кэш.
* Служба сообщает версию и SHA-256; CLI заменяет несовпадающую службу штатно.
  Идентичность исполняемого файла запоминается до загрузки профиля, поэтому
  удаление его пути через cleanup не изменяет отчёт работающей службы.
* Управление запуском, заменой и удалением сериализовано блокировкой каталога.
* daemon --uninstall останавливает все профили нужных корней, выгружает
  LaunchAgents и удаляет только кэш приложения. Не открывает ключи профилей.
  По умолчанию включаются все корни из Shum LaunchAgents; --data-dir ограничивает
  команду одним каталогом.
* README, документация и проект заметок описывают удаление и ручную очистку.
  Из локального проекта заметок убран шаг Ctrl+Q при обновлении; опубликованные
  заметки GitHub пока не редактировались.

## Вывод проверок

```text
cargo test --offline --locked
29 passed; 0 failed

cargo clippy --offline --locked --all-targets -- -D warnings
Finished dev profile (без предупреждений)

cargo fmt --check
PASS

git diff --check
PASS

ruby -c <проект формулы>
Syntax OK

verify_macos_package.py
PASS: signed Shum.app, versioned plist, installer symlink, receipt, archive and checksums
PASS: CLI --version and --help with only system executables in PATH

macos_service_lifecycle.py (два временных File-key профиля)
PASS real launchctl bootstrap/bootout, KeepAlive=false, two profiles, idempotent uninstall
PASS profile registry, key files and encrypted database records preserved
PASS packaged app reused directly; LaunchAgent executable outside Cellar

macos_bundle_upgrade.py (изолированная структура Cellar/opt)
PASS same certificate: direct bundle reuse, stable opt path, cleanup of loaded Cellar version
PASS changed hash replaces PID automatically; profile identity and history preserved
PASS uninstall removes temporary LaunchAgent and preserves profile files

service_upgrade.py (старая служба без поля build)
PASS automatic service upgrade: graceful exit, stable identity and saved profile

command_colors.py
PASS Apple_Terminal и WarpTerminal, ошибки, цвета, фильтры, --json и NO_COLOR
```

Интеграционный тест cli_relay заменяет службу бинарником с другим SHA-256 при
той же версии, сохраняет ожидающее сообщение со статусом forwarding и проверяет
его последующую доставку, когда получатель снова подключается. В тестах используются
только локальные серверы и временные профили, без Bluetooth и внешних релеев.

## Проверка 1: одинаковая подпись разных сборок

У Release и Debug бинарников разные SHA-256, а codesign -dr - возвращает одинаковое
требование. Обе сборки проходят codesign --verify --strict.

```text
designated => identifier "org.shum.cli" and certificate leaf = H"9ce51b5c3430742e7860fc77583b41baa8355085"
```

SHA-256 подписанного Release: `5f3613e05b8ca273282aa5b1e3a572d06e00db9453a41b750a922c519e5311d9`.

SHA-256 подписанного Debug: `b3b9545292af2abe0c846ba04371533916aaa71c7cdddbd6867ea06429505d07`.

Хэша исполняемого файла (cdhash) в требовании нет. H в certificate leaf является
отпечатком публичного сертификата.

## Проверка 2: настоящий Homebrew N → N+1 с Bluetooth

**Не выполнена.** Публичный tap и установленная формула не обновлялись. На временном
префиксе проверены смена подписанного бандла, стабильный opt-путь, автоматический
перезапуск и удаление старой папки Cellar. Это не является проверкой brew upgrade
или разрешения Bluetooth. Обе тестовые сборки имеют версию 0.1.5 и различаются
исполняемыми файлами; обновление разных версий требует отдельного прогона.

Нет оснований обещать, что macOS не спросит Bluetooth повторно. Настоящий прогон
с обменом сообщениями нужен после согласования обновления и с участием владельца
iPhone. Если запрос появится снова, это будет зафиксировано как отрицательный
результат, без обходов TCC.

## Проверка 3: удаление службы и сохранность данных

**Команда проверена на настоящем launchd, на временных профилях.** После удаления
launchctl print не находит их служб, файлов plist и daemon.json нет, кэш удалён.
Оба профиля остаются в реестре, файлы ключей и базы существуют; зашифрованные записи
совпадают с состоянием до удаления. WAL после штатной остановки пуст; повторная
команда успешна, даже если каталог данных отсутствует.

**Полный прогон shum daemon --uninstall → brew uninstall shum на установленной
формуле не выполнен.** Рабочую установку пользователя не удаляли. Этот шаг нужно
провести отдельно при согласованном тесте установки. Ручная очистка при удалённой
формуле описана в README; KeepAlive=false подтверждён реальными plist тестов.

## Локальные артефакты

Проверочный Release: `/tmp/shum-signed-verified-release`.
Содержит архив, .pkg, SHA-256, JSON и `homebrew-shum/Formula/shum.rb`.

Проверочный Debug: `/tmp/shum-signed-verified-debug`.

Эти артефакты имеют текущую версию 0.1.5 и не предназначены для замены уже
опубликованного выпуска 0.1.5. Перед публикацией потребуется новый номер версии,
пересборка, проверка и согласование. URL формулы пока локальный file://.
