# Проверки универсальной сборки и установки скриптом

Состояние на 9 октября 2026. Mac владельца: Apple Silicon, macOS 15.6.1.
Первоначальные проверки выполнены до синхронизации исходников, без публикации
и изменения установленной формулы.
После согласия владельца установлены Intel-цель Rust и ShellCheck 0.11.0.
Реализация, универсальная сборка и локальные проверки завершены. Приёмочные
проверки чистой системной учётной записи, Gatekeeper и Bluetooth остаются открытыми.
Проверочная версия N+1 создана только во временной копии исходников; Cargo.toml
рабочего репозитория остаётся 0.1.5. Выпуск 0.1.6 не публиковался.

## Что подготовлено

* package_macos.py собирает две цели Rust, объединяет lipo, подписывает один
  бинарник внутри Shum.app сертификатом Shum CLI Signing. Версия из Cargo.toml.
* Временный адаптер xcrun передаёт Swift архитектуру: встроенный Bluetooth-помощник
  должен совпадать с архитектурой каждого CLI-среза. Код ядра и Cargo cache не меняются.
* Mach-O проверяется по каждому срезу: CLI, встроенный помощник, minimum macOS 15.0.0,
  только системные dylib. Проверяются одинаковые сертификатные DR обеих архитектур.
* Архив и checksum имеют стабильные имена `shum-macos-universal.tar.gz` и
  `shum-macos-universal.tar.gz.sha256`. Черновик формулы не ограничен arm64;
  используется архив конкретного выпуска. Живой tap не изменён.
* install.sh: POSIX sh, macOS arm64/x86_64, без sudo; releases/latest/download,
  SHA-256, whitelist архива без ссылок, подпись с закреплённым публичным сертификатом,
  версия, обе архитектуры. Ошибки до переключения сохраняют рабочую установку.
* Установка в ~/.local/share/shum/versions/<SHA256>/Shum.app, стабильные App/bin ссылки,
  PATH подсказка без изменения shell-файлов. Отказ рядом с Homebrew и при чужом bin/shum.
* Исправлено распознавание bin/shum: macOS может вернуть путь ссылки, поэтому
  перед проверкой App он canonicalize. Без этого реальная установка уходила
  в Cargo fallback и переподписывала службу ad hoc. Это выявил полный тест
  установки; исправление покрыто регрессией и повторным полным сценарием.
* Внутренний daemon --refresh обновляет только работающие службы. Не создаёт профиль
  и не открывает ключи неактивных профилей. LaunchAgent использует стабильный App путь.
* --uninstall сначала штатно удаляет службы, затем файлы установки; сохраняет данные.
* README, distribution и заметки описывают новый формат, команды после публикации,
  ограничения самоподписанного сертификата и отсутствие проверки настоящего Intel Mac.

## Автоматические проверки

`cargo test --locked --offline`: **31 passed, 0 failed**, повторено после исправления bin/shum.
Среди них релейные приглашения, сообщения, квитанции и замена службы с сохранением
очереди; новый тест refresh не открывает неактивные/устаревшие профили.

`cargo clippy --locked --offline --all-targets -- -D warnings`:

```text
Checking shum-cli v0.1.5
Finished `dev` profile [unoptimized]
```

`sh -n install.sh`, целевой rustfmt --check, Python compile и git diff --check: exit 0.

`python3 tests/manual/install_script.py`:

```text
PASS: unsupported OS; Homebrew refusal; SHA mismatch; signature rejection; PATH output
PASS: install/repeat/Intel update; stable symlink; immutable old App; refresh invocation
PASS: failed update preserves installation; idempotent removal preserves data; foreign files protected
Fixtures only: no Gatekeeper, Bluetooth, real launchd or real release downloads exercised
```

В этом тесте curl и инспекция подписи заменены локальными фикстурами, два фиксированных
пути поиска Homebrew перенесены во временный каталог. SHA-256, tar, файловая установка,
атомарное переключение ссылки и удаление выполняются настоящими системными командами.
Строка «Intel update» обозначает ветку uname=x86_64, а не запуск настоящего Intel CLI.

`python3 tests/manual/package_architectures.py`:

```text
PASS: universal slices, per-architecture macOS minima, matching embedded BLE
PASS: thin input, wrong helper architecture, newer minimum OS and missing helper rejected
```

Это синтетические Mach-O для регрессии парсера; универсальный выпуск не заменяют.

## Универсальные артефакты и C-зависимости

Сохранены две локальные подписанные сборки:

* `/tmp/shum-universal-fixed-signed`: рабочая версия 0.1.5.
* `/tmp/shum-universal-next-fixed-signed`: локальный тест N+1 (0.1.6), отдельная
  временная копия исходников, без повышения версии репозитория и публикации.

В каждом каталоге универсальный архив, SHA-256, .pkg, JSON и черновик формулы.
Контейнер .pkg без Developer ID Installer остаётся `-unsigned`; App внутри подписан.
У каждого CLI-среза есть свой соответствующий Bluetooth-помощник, всего четыре Mach-O.
`minimumMacOSByArchitecture`: arm64=15.0.0, x86_64=15.0.0.

```text
Architectures in the fat file: .../Shum.app/Contents/MacOS/shum are: x86_64 arm64
```

DR обеих архитектур и обеих разных сборок совпадает:

```text
designated => identifier "org.shum.cli" and certificate leaf = H"9ce51b5c3430742e7860fc77583b41baa8355085"
```

SHA-256 бинарников различается:

```text
N:   c488bf390bd62e43a6c23e8952a93665a22107b47811cce3f6570fb22903d300
N+1: 55c5f02ea518ffecf3089129a9c0fcb6c8d9e773cd9120c5929640594b90e988
```

`verify_macos_package.py` прошёл для обоих окончательных .pkg:

```text
PASS: signed Shum.app, versioned plist, installer symlink, receipt, archive and checksums
PASS: CLI --version and --help with only system executables in PATH
Installer inspected, not installed. Existing profiles not accessed.
```

`ruby -c` черновика формулы: `Syntax OK`. Ограничения arch: arm64 нет.

secp256k1-sys, libsqlite3-sys и aws-lc-sys собраны под оба Rust target;
`lipo -info` их .a подтверждает соответственно arm64 и x86_64.
libz-sys использует системный zlib (`cargo:rustc-link-lib=z`), а не внешний
Homebrew-бинарник. `otool -L` у обоих CLI-срезов показывает `/usr/lib/libz.1.dylib`;
все остальные динамические зависимости также из /usr/lib или /System/Library.

## Результаты требуемых проверок 1–6

| № | Результат |
|---|---|
| 1 | Пройдено: подписанный Shum.app содержит x86_64 и arm64; minimum macOS 15.0 обеих архитектур; DR без cdhash, одинаковый между архитектурами и двумя разными сборками. |
| 2 | Пройдено под Rosetta: arch -x86_64 бинарник из App --version возвращает shum 0.1.5. Intel-релейный тест после окончательного исправления: 1 passed, 0 failed (23.89s). Приглашения, сообщения, квитанции, перезапуск и сохранение очереди. Настоящего Intel Mac не было. |
| 3 | Не выполнено на чистой системной учётной записи: временный HOME не меняет macOS user/TCC/Keychain. Отсутствие предупреждения Gatekeeper и единственный запрос Bluetooth не подтверждены. Самоподписанный сертификат не позволяет обещать это. |
| 4 | Локальный N → N+1 пройден полным install.sh: новая версия и хэш обеих служб, PID меняются, профиль и история сохраняются. Повтор той же версии сохраняет PID. Реального нового GitHub-выпуска не было; Bluetooth был выключен в тестовых профилях, отсутствие повторного запроса не подтверждено. |
| 5 | Пройдено с настоящим подписанным архивом: неверный SHA-256 отклоняется при первой установке и обновлении; работающий пакет сохраняется. Отдельно проверены отказ подписи, чужих файлов и установки рядом с Homebrew. |
| 6 | Пройден полный install.sh --uninstall с настоящими пакетами и launchd, на двух временных профилях: нет их служб/LaunchAgents, App и bin-ссылки удалены, registry/keys/DB records сохранены; повтор без ошибок. Пользовательские профили не затрагивались. |

## Полный сценарий установщика

`tests/manual/macos_installer_lifecycle.py` использует реальные подписанные архивы,
codesign, CLI, файловую установку и launchd. Только загрузка assets, команда brew
и два глобальных Homebrew пути заменены локальными фикстурами, чтобы не выпускать
пакет и не удалять установленный у владельца Homebrew CLI.
Подсказка PATH проверена; .zshrc/.zprofile не создавались.
После остановки проверен checkpoint WAL, затем сохранённые зашифрованные DB records
прочитаны в режиме immutable без создания WAL/shm.

```text
shum 0.1.5
PASS actual signed universal App: shum 0.1.5 -> shum 0.1.6, SHA checks, Rosetta --version
PASS repeat install keeps PID; upgrade refreshes both real LaunchAgents; identity/history preserved
PASS incorrect SHA aborts first install/update; full --uninstall removes App/bin/agents and preserves DB/keys
Local release fixtures, existing macOS account with temporary HOME: Gatekeeper/Bluetooth/real releases not checked
```

Настоящий launchd, `tests/manual/macos_user_install.py`:

```text
PASS real launchd: stable ~/.local App path, KeepAlive=false, bundle reused directly
PASS installer refresh changes daemon PID/hash, preserves identity/history, skips inactive profiles
PASS bootout, agent removal, idempotent uninstall, profile files retained
Temporary local ad-hoc arm64 fixtures; no universal, Gatekeeper, Bluetooth or release check
```

Повторная проверка `macos_service_lifecycle.py` после изменений:

```text
PASS real launchctl bootstrap/bootout, KeepAlive=false, two profiles, idempotent uninstall
PASS profile registry, key files and encrypted database records preserved
Only disposable profiles uninstalled; installed Homebrew formula unchanged
```

## ShellCheck

ShellCheck 0.11.0 установлен по явному согласию владельца.
Окончательный запуск:

```sh
shellcheck --shell=sh install.sh
```

**Exit 0, stdout и stderr пустые, замечаний нет.** Первоначальное SC2016 в
буквальной подсказке PATH устранено экранированием переменных в двойных кавычках;
содержание печатаемой команды не изменилось. Отключений правил ShellCheck нет.

## Сохранность чужих изменений

tests/tui.rs не изменён, SHA-256:
`6e90abd725b58572c4807abca1ea3ec919feb3641cd346906fa14f43e3a8538e`.
Ранее изменённые src/ui.rs и src/terminal.rs также сохранены байт в байт.
Профили пользователя, доверие сертификатов, TCC и ключи не менялись.
Пароли не вводились, приватный ключ не экспортировался.

## Ограничение подписи

[Apple: Developer ID](https://developer.apple.com/developer-id/) и
[Gatekeeper](https://support.apple.com/en-us/102445) описывают проверку разработчика
и нотарификации. Самоподписанный сертификат не является Developer ID.
Установщик не меняет доверие, не отключает Gatekeeper и не удаляет quarantine.
Если при проверке обновления macOS снова запросит Bluetooth, результат следует
зафиксировать как неуспех требования; сейчас такой результат не получен.

Для неподдерживаемых ОС install.sh пока ссылается на публичную страницу проекта
https://github.com/hiTechTeam/Shum-CLI. Адрес отдельного сайта не был предоставлен;
его можно заменить в project_url после уточнения.

## Проверка стабильного пути Homebrew

`macos_bundle_upgrade.py` с двумя окончательными подписанными универсальными архивами:

```text
PASS same certificate: direct bundle reuse, stable opt path, cleanup of loaded Cellar version
PASS changed hash replaces PID automatically; profile identity and history preserved
PASS uninstall removes temporary LaunchAgent and preserves profile files
Homebrew itself and Bluetooth were not exercised
```

Временный старый Cellar удалялся при работающей службе, новая CLI-команда штатно
заменила её через opt путь. Живой tap и установленная формула владельца не менялись.
