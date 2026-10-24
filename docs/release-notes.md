# Shum CLI 0.1.17 · 24 October 2026

* The same Shum-Core as every other client: the iPhone now runs the same Bluetooth mesh as Shum on computers, so a CLI and an iPhone nearby find each other and talk over Bluetooth directly.
* Installed 0.1.16 updates to this version on its own.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.12 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* То же ядро Shum-Core, что у всех клиентов: iPhone теперь работает на той же Bluetooth-сети, что Shum на компьютерах, поэтому CLI и iPhone рядом находят друг друга и переписываются по Bluetooth напрямую.
* Установленная 0.1.16 обновится до этой версии сама.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.12 те же изменения.

# Shum CLI 0.1.16 · 24 October 2026

* Delivery marks as in every Shum client: ◷ while a message is on its way (queued or waiting on a relay), ✓ once the recipient's device has it, ✓✓ once it is read. ASCII mode shows [sending], [delivered], [read].
* Installed 0.1.15 updates to this version on its own.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.11 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Отметки доставки как во всех клиентах Shum: ◷ пока сообщение в пути (в очереди или ждёт на релее), ✓ когда оно на устройстве получателя, ✓✓ когда прочитано. В режиме ASCII: [sending], [delivered], [read].
* Установленная 0.1.15 обновится до этой версии сама.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.11 те же изменения.

# Shum CLI 0.1.15 · 23 October 2026

* Shum updates itself: the background service checks GitHub once a day and installs a new version the way Shum was installed (Homebrew or the install script), then restarts on it. `shum update` updates at once; `shum update --auto off` turns the background updates off.
* Shum-Core 0f747a1, as in 0.1.14.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.10 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Shum обновляется сам: фоновая служба раз в сутки проверяет GitHub, ставит новую версию тем же способом, каким Shum был установлен (Homebrew или скрипт установки), и перезапускается на ней. `shum update` обновляет сразу, `shum update --auto off` выключает обновление в фоне.
* Shum-Core 0f747a1, как в 0.1.14.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.10 те же изменения.

# Shum CLI 0.1.14 · 23 October 2026

* Push notifications go to the new service at push.getshum.tech, which also works behind a VPN. Profiles that stored the old address switch to it on their own.
* An answer to an invitation wakes the contact's iPhone silently instead of showing another invitation notification.
* Shum-Core 0f747a1, the same core as the desktop app 0.1.4 and Shum Web.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.9 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Push-уведомления идут в новый сервис push.getshum.tech, он работает и через VPN. Профили со старым адресом переключаются на него сами.
* Ответ на приглашение будит iPhone собеседника тихо, без ещё одного уведомления о приглашении.
* Shum-Core 0f747a1, то же ядро, что в приложении для компьютера 0.1.4 и Shum Web.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.9 те же изменения.

# Shum CLI 0.1.13 · 20 October 2026

* Shum-Core 6e41fbc, the same core as the desktop app 0.1.3, Shum Web and the iPhone app in development.
* People met nearby over Bluetooth are kept in the profile the way the iPhone keeps them (24 hours), so a backup carries them between devices.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.8 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Shum-Core 6e41fbc, то же ядро, что в приложении для компьютера 0.1.3, Shum Web и разрабатываемой версии для iPhone.
* Люди, встреченные рядом по Bluetooth, хранятся в профиле так же, как на iPhone (24 часа), поэтому резервная копия переносит их между устройствами.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.8 те же изменения.

# Shum CLI 0.1.12 · 11 October 2026

* One core for every Shum client: the profile session and the relay connections are now the same Shum-Core code in the CLI, the desktop app and the new browser version, Shum Web (app.getshum.tech).
* Relays reconnect the same way everywhere: after a failed attempt Shum waits twice as long each time, up to a minute, and keeps up to 64 messages per relay while it is offline.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.7 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Одно ядро для всех клиентов Shum: сессия профиля и соединения с релеями теперь один и тот же код Shum-Core в CLI, приложении для компьютера и новой браузерной версии Shum Web (app.getshum.tech).
* Релеи переподключаются везде одинаково: после неудачной попытки Shum ждёт каждый раз вдвое дольше, но не больше минуты, и держит до 64 сообщений на релей, пока тот недоступен.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.7 те же изменения.

# Shum CLI 0.1.11 · 11 October 2026

* "In chat" mark, as in the iOS app: a 3x3 pixel grid next to the name in the chat list and before the status in the chat header. It pulses while the contact types; in ASCII mode the word "typing…" stays.
* Distance nearby on both sides: the device that measures the Bluetooth signal now shares its reading, so a Mac that a peer connected to also shows "~N m" (protocol section 06, signal report).
* New profiles are created by Shum-Core, the same way as in the desktop app.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.6 carries the same changes.

```sh
brew update
brew upgrade shum
```

## Русский

* Знак «в чате», как в приложении для iPhone: сетка 3×3 пикселя рядом с именем в списке чатов и перед подписью в шапке чата. Пока собеседник печатает, она шевелится; в режиме ASCII остаётся слово «печатает…».
* Расстояние «рядом» видно с обеих сторон: устройство, которое измеряет сигнал Bluetooth, теперь сообщает замер соседу, поэтому Mac, к которому подключились, тоже показывает «~N м» (раздел 06 протокола, отчёт о сигнале).
* Новые профили создаёт Shum-Core, так же как в приложении для компьютера.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.6 те же изменения.

# Shum CLI 0.1.10 · 11 October 2026

* The CLI works alongside the Shum desktop app: when the app has a profile open, the CLI joins it instead of restarting it.
* Avatars and reaction pictures now come from Shum-Core, so every Shum client draws them the same way.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.5 carries the same change.

```sh
brew update
brew upgrade shum
```

## Русский

* CLI работает вместе с приложением Shum для компьютера: если профиль открыт в приложении, CLI подключается к нему и не перезапускает его.
* Аватарки и картинки реакций теперь рисует Shum-Core, поэтому во всех клиентах Shum они одинаковые.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.5 то же изменение.

# Shum CLI 0.1.9 · 11 October 2026

* Profile backups: `shum backup create <file>` saves the selected profile to a password-protected file, `shum backup restore <file>` brings it back as a new profile with the same Shum ID. The format is protocol section 13, shared by all Shum clients through Shum-Core.
* The first-run profile screen points to `shum backup restore` for people who already have a backup.

macOS 15+, Apple Silicon and Intel. Windows preview 0.1.4 carries the same change.

```sh
brew update
brew upgrade shum
```

## Русский

* Резервные копии профиля: `shum backup create <файл>` сохраняет выбранный профиль в файл с паролем, `shum backup restore <файл>` возвращает его как новый профиль с тем же Shum ID. Формат описан в разделе 13 протокола и общий для всех клиентов через Shum-Core.
* Экран создания профиля при первом запуске подсказывает `shum backup restore` тем, у кого уже есть копия.

macOS 15+, Apple Silicon и Intel. В превью для Windows 0.1.4 то же изменение.

# Shum CLI 0.1.8 · package revision 1

* Avatars and reaction icons remain visible outside every menu, including profiles, help, QR and confirmation dialogs.
* Profile switching waits for the new profile data instead of showing an empty frame.
* Delivery status updates no longer repeatedly clear the Warp screen. Graphics and text redraw together.

App version: 0.1.8. Homebrew package: 0.1.8_1. macOS 15+, Apple Silicon and Intel.

```sh
brew update
brew upgrade shum
```

Reopen the chat interface after upgrading. The service refreshes automatically.
For script installations, run `install.sh` again. Profiles and messages are preserved.

40 tests and Clippy passed. The owner confirmed the signed preview works in Warp.
The published binary is identical to that preview. The app is self-signed and not
notarized; the pkg container is unsigned. Actual Intel hardware and Bluetooth
permission persistence were not retested for this revision.

## Русский

* Аватары и реакции остаются видимыми вне всех меню, включая профили, справку, QR и подтверждения.
* Переключение профиля дожидается новых данных, без пустого промежуточного кадра.
* Обновления статуса доставки больше не очищают экран Warp несколько раз. Графика и текст перерисовываются вместе.

Версия CLI 0.1.8, пакет Homebrew 0.1.8_1. Обновление: `brew update`, затем
`brew upgrade shum`. После обновления откройте интерфейс чатов заново.
Для установки скриптом повторите `install.sh`. Профили и переписка сохраняются.

Прошли 40 тестов и Clippy. Владелец подтвердил работу подписанной сборки в Warp;
опубликован тот же бинарник. Настоящий Intel Mac и сохранение разрешения Bluetooth
для этой ревизии повторно не проверялись.

Удаление: `shum daemon --uninstall`, затем `brew uninstall shum`. Данные остаются
в `~/Library/Application Support/org.Shum.Shum` или вашем `--data-dir`.

# Shum CLI 0.1.8 · 10 October 2026

* Ten interface languages: English, Russian, Spanish, Simplified Chinese, Hindi,
  French, Japanese, Brazilian Portuguese, Arabic and Korean. System language is
  detected automatically; the chosen language is saved locally.
* `/language` and Ctrl+L open a language menu. Use arrows and Enter, or set a
  language directly with `/language en` or `shum language en`.
* Ctrl+R opens the message and pixel reaction picker, alongside `/react`.
  Cancelling or applying a reaction preserves the message draft.
* Avatars and reaction icons stay visible outside reaction and language menus.
  Reaction PNGs in Warp are smaller.
* Invitation requests, acceptance and decline appear in the chat history and
  survive restarts. The recipient uses `/accept` or `/decline` and can still
  accept after declining. Declined incoming invitations remain in the invitations tab.
* New invitation events are saved with the encrypted profile state. Protocol
  formats and the pinned Core dependency are unchanged.

Update with `brew update`, then `brew upgrade shum`. Reopen the chat interface
for the new controls. The next CLI command updates the service automatically,
preserving profiles, history and the outgoing queue. Script installations update
by running `install.sh` again.

macOS 15+, Apple Silicon and Intel. The app uses the existing self-signed
certificate and is not notarized; the pkg container is unsigned. Actual Intel
hardware, clean-account Gatekeeper and Bluetooth permission persistence after
this release have not been verified.

Uninstall: `shum daemon --uninstall`, then `brew uninstall shum`. Profiles,
keys and messages are preserved. Data lives in
`~/Library/Application Support/org.Shum.Shum` or your `--data-dir`.
To erase profiles and their keys too, run `shum daemon --uninstall --purge`
and confirm `DELETE` before removing the package.

## Русский

* Десять языков интерфейса: русский, английский, испанский, упрощённый китайский,
  хинди, французский, японский, бразильский португальский, арабский и корейский.
  Язык системы определяется автоматически, выбранный язык сохраняется.
* `/language` и Ctrl+L открывают меню языка. Выбор стрелками и Enter.
  Также работают `/language en` и `shum language en`.
* Ctrl+R открывает выбор сообщения и пиксельной реакции, как `/react`.
  Черновик сохраняется после выбора или отмены реакции.
* Аватары и реакции остаются видимыми вне окон выбора реакции и языка.
  PNG-реакции в Warp стали компактнее.
* Отправка приглашения, принятие и отказ видны в переписке и сохраняются после
  перезапуска. Получатель выбирает `/accept` или `/decline` и может принять
  приглашение после отказа. Отклонённый входящий запрос остаётся во вкладке приглашений.
* Новые события приглашений сохраняются вместе с зашифрованным состоянием
  профиля. Форматы протокола и закреплённая версия ядра не изменены.

Обновление: `brew update`, затем `brew upgrade shum`. Откройте интерфейс чатов
заново. Служба обновится автоматически при следующей команде CLI, сохраняя
профили, историю и очередь отправки. Для установки скриптом повторите `install.sh`.

macOS 15+, Apple Silicon и Intel. Приложение подписано прежним самоподписанным
сертификатом, нотарификации нет; контейнер pkg не подписан. Настоящий Intel Mac,
Gatekeeper на чистой учётной записи и сохранение разрешения Bluetooth после
этого выпуска не проверены.

Удаление: `shum daemon --uninstall`, затем `brew uninstall shum`. Профили,
ключи и переписка сохраняются. Данные находятся в
`~/Library/Application Support/org.Shum.Shum` или вашем `--data-dir`.
Для стирания профилей вместе с ключами выполните
`shum daemon --uninstall --purge`, подтвердите `DELETE`, затем удалите пакет.

---

# Shum CLI 0.1.7, package revision 2 · 10 October 2026

* Chat history shows pixel reaction icons with their authors' names.
* Matching reactions share one icon and both names separated by `/`.
  Different reactions each show their icon and author, separated by `/`.
* Warp uses compact PNG icons. Terminal uses the same pixel artwork in coloured cells.
* Reactions remain attached to their messages while scrolling and resizing;
  modal windows cover the chat's PNG images.
* App version remains 0.1.7. Homebrew package version is 0.1.7_2.

Update with `brew update`, then `brew upgrade shum`. Reopen the chat interface
for the new rendering. The service updates automatically on the next CLI command,
preserving profiles, history and the outgoing queue. Script installations update
by running `install.sh` again.

macOS 15+, Apple Silicon and Intel. Self-signed app, not notarized;
the pkg container is unsigned. Actual Intel hardware was not tested.

## Русский

* В переписке вместо названий реакций показаны пиксельные иконки с именами авторов.
* У одинаковой реакции одна иконка и два имени через `/`.
  У разных реакций своя иконка и имя автора, разделённые `/`.
* В Warp используются компактные PNG, в Terminal те же рисунки из цветных ячеек.
* Прокрутка, изменение размера и открытие окон учитывают расположение иконок.
* Версия приложения осталась 0.1.7, пакет Homebrew: 0.1.7_2.

Обновление: `brew update`, затем `brew upgrade shum`. Откройте интерфейс чатов
заново. Служба обновится автоматически при следующей команде CLI с сохранением
профилей, переписки и очереди отправки. Для установки скриптом повторите `install.sh`.

macOS 15+, Apple Silicon и Intel. Приложение подписано прежним самоподписанным
сертификатом, нотарификации нет; контейнер pkg не подписан.
На настоящем Intel Mac не проверялось.

---

# Shum CLI 0.1.7, package revision 1 · 10 October 2026

English · Русский below

* `/react` opens message selection with arrow keys, then a pixel reaction picker.
  Enter confirms; Esc goes back or cancels. Selecting your current reaction removes it.
* Profile avatars appear in the Warp Ctrl+P window.
* Nearby peers without a signal estimate show that distance is unknown.
* App version remains 0.1.7. Homebrew package version is 0.1.7_1.
  Release assets are published under `v0.1.7-r1`; the original archive is preserved.

Update with `brew update`, then `brew upgrade shum`. Script installations use
`install.sh` again. The next CLI command replaces an outdated service by its
binary hash while preserving profiles and the outgoing queue. Reopen the chat
interface to use the new controls.

macOS 15+, Apple Silicon and Intel. The app uses the same self-signed certificate.
It is not notarized; the pkg container is unsigned. A real Intel Mac was not tested.

## Русский

* `/react` открывает выбор сообщения стрелками, затем меню пиксельных реакций.
  Enter подтверждает, Esc возвращает назад или отменяет. Повторный выбор снимает реакцию.
* В окне Ctrl+P в Warp снова отображаются аватары профилей.
* Если нет оценки сигнала, рядом с собеседником написано «расстояние неизвестно».
* Версия приложения остаётся 0.1.7. Ревизия пакета Homebrew: 0.1.7_1.
  Артефакты опубликованы под `v0.1.7-r1`, исходный архив сохранён.

Обновление: `brew update`, затем `brew upgrade shum`. При установке скриптом
повторите `install.sh`. Следующая команда CLI заменит устаревшую службу по хэшу
бинарника, сохраняя профили и очередь отправки. Чтобы увидеть новые элементы
интерфейса, откройте чаты заново.

macOS 15+, Apple Silicon и Intel. Сертификат подписи прежний, самоподписанный.
Нотарификации нет, контейнер pkg не подписан. На настоящем Intel Mac не проверялось.

---

# Shum CLI 0.1.7 · 10 октября 2026

* Регистрация: анимация сборки ключа и щита, плавная шкала и галочки после
  проверки этапов. Затем имя и первый аватар из открытого ключа подписи.
* Чёткие PNG-аватары в Warp: 576×576, целочисленное увеличение без сглаживания.
* Имена профилей могут совпадать. Выбор и удаление используют полный ID из
  `shum profile list`; подтверждение именем не выбирает цель удаления.
* Полное удаление: `shum daemon --uninstall --purge`, подтверждение `DELETE`,
  затем `brew uninstall shum`. Удаляются профили, ключи, история и кэш служб.
  Обычная команда `shum daemon --uninstall` сохраняет данные.
* Ядро закреплено на `93df231`. Форматы протокола не менялись.

macOS 15+, Apple Silicon и Intel. Универсальный Shum.app подписан прежним
сертификатом Shum CLI Signing. Самоподписанная подпись не заменяет Developer ID
или нотарификацию. Контейнер pkg не подписан. На настоящем Intel Mac проверки
не было; Rosetta не заменяет такую проверку.

Обновление: `brew update`, затем `brew upgrade shum`. Служба обновляется
автоматически при следующем запуске CLI, сохраняя очередь отправки.
При установке скриптом повторите команду установки из README.

Данные: `~/Library/Application Support/org.Shum.Shum` или ваш `--data-dir`.
Без `--data-dir` purge также учитывает корни из LaunchAgents Shum. Посторонние
файлы сохраняются. Если приложение уже удалено, порядок ручной очистки есть в README.

[Проверки 0.1.7](release-verification-0.1.7.md).

---

# Shum CLI 0.1.6 · 9 октября 2026

* Универсальный пакет: macOS 15+, Apple Silicon и Intel. Один подписанный
  бинарник содержит оба среза, включая встроенного Bluetooth-помощника.
  На Rosetta проверены запуск и релейный сценарий; на настоящем Intel Mac
  пока не проверялось.
* Установка и обновление без Homebrew через install.sh: проверка SHA-256,
  стабильный путь в ~/.local, мягкое обновление активных служб.
  Удаление: `sh install.sh --uninstall`; данные сохраняются.
* Shum.app входит в архив и подписывается постоянным сертификатом Shum CLI Signing.
  Версия Info.plist соответствует версии пакета.
* Homebrew устанавливает бандл и ссылку на команду; автозапуск использует opt/shum.
* При следующем запуске CLI служба автоматически заменяется при изменении версии
  или SHA-256, с сохранением очереди отправки.
* Перед удалением выполните `shum daemon --uninstall`, затем `brew uninstall shum`.
  Данные профилей и ключи сохраняются.

Данные macOS: `~/Library/Application Support/org.Shum.Shum` или указанный `--data-dir`.
Для полного стирания остановите службы и удалите каталог вручную; ключи профилей
в Keychain удаляются отдельно. Если формула уже удалена, выгрузите каждый
`~/Library/LaunchAgents/org.shum.cli.<ID_профиля>.plist` через `launchctl bootout`,
затем удалите файл. KeepAlive=false не допускает бесконечного перезапуска.

На Mac владельца проверены настоящий brew upgrade с 0.1.5 на 0.1.6,
автоматическая замена службы и сообщения с iPhone в обе стороны.
При переходе с прежней ad hoc подписи на сертификатную macOS запросила
Bluetooth заново; владелец разрешил доступ. Отсутствие повторного запроса
между двумя сертификатными выпусками ещё требует проверки.

Самоподписанный сертификат не является Developer ID. Проверки чистой учётной
записи и Gatekeeper пока не выполнены. [Отчёт о проверках](https://github.com/hiTechTeam/Shum-CLI/blob/main/docs/release-verification.md).
Установка: `brew install hitechteam/shum/shum`. Обновление: `brew update`,
затем `brew upgrade shum`. Для установки без Homebrew:

```sh
curl -fsSL https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh | sh
```

* Команды контактов используют Shum ID или полный сетевой ID, имена служат
  для отображения. В чате `/invite` приглашает текущего собеседника; `/qr`
  показывает собственный QR. Полная карточка и QR-файл добавляются офлайн.
* Исправлены прокрутка истории, компактные аватары Terminal, очистка PNG
  при переключении чатов в Warp и индикатор набора текста в списке чатов.
* MIT LICENSE добавлен в CLI, Core и Protocol. В бандле есть текст лицензии.
* CLI использует Shum-Core `4020dfd` и черновик v1. Устройства и синхронизация
  будущей v1 stable пока не реализованы. Intel проверен под Rosetta;
  это не заменяет проверку настоящего Intel Mac.
