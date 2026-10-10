# Shum CLI 0.1.8, package revision 1

Verified on 10 October 2026. App version: `0.1.8`.
Homebrew package: `0.1.8_1`. Release tag: `v0.1.8-r1` in Shum-CLI and homebrew-shum.

## Checks

* 40 targeted tests passed: menu image coverage, nested profile confirmations,
  message delivery redraws, reactions, languages, invitation timeline and TUI navigation.
* Clippy for all targets with warnings denied, formatting and diff checks passed.
* The owner confirmed the signed local preview works in Warp. The release archive
  and installer are byte-identical to that preview; only the package revision,
  source metadata and public formula URL changed.
* Package verification passed: signatures, versioned plist, archive and installer
  checksums, Homebrew revision, both architecture slices and embedded BLE helpers.
* Both architectures require macOS 15.0.0. Rosetta `--version` returned `shum 0.1.8`.
* Disposable-profile upgrade from 0.1.8 passed: the changed hash replaced the service
  PID automatically through the stable opt path, preserving identity and history.
  Removing the old Cellar directory did not break startup. Uninstall removed the
  temporary LaunchAgent and preserved profile data.
* Certificate requirement remains `identifier "org.shum.cli" and certificate leaf =
  H"9ce51b5c3430742e7860fc77583b41baa8355085"`, without a binary hash.

Binary SHA-256: `81579062ecd6a411ca3bed35156b55727659e19154303d88edcb40eb51590fb9`.
Archive SHA-256: `6251ad720b3a5bb12b27f650f65c625821fdf0f9c4bccf68168b6a30e6ca0dd8`.

## Update

```sh
brew update
brew upgrade shum
```

Reopen the chat interface after upgrading. Script installations update by running
`install.sh` again. Previous release assets remain available. Automatic CI remains
manual. The owner's installed application was not changed during verification.

## Limits

Homebrew installation itself, Bluetooth permission persistence, iPhone exchange,
clean-account Gatekeeper and actual Intel hardware were not retested. The upgrade
check uses a disposable Homebrew-style directory layout; Rosetta is not real Intel
hardware. The app is self-signed, not notarized; the installer container is unsigned.
No new profiles or keys were created in the owner's data directory.

## Русский

Версия приложения 0.1.8, пакет Homebrew 0.1.8_1. Прошли 40 тестов, Clippy,
проверка подписанного пакета и запуск через Rosetta. Владелец подтвердил исправление
в Warp, опубликованы те же бинарник и архив.

На временном профиле проверены автоматическая замена службы по новому хэшу,
сохранение истории и идентификатора, стабильный путь opt и удаление тестового
LaunchAgent с сохранением данных. Пользовательские профили не затронуты.

Настоящий Intel Mac, установка через Homebrew, Bluetooth, обмен с iPhone и Gatekeeper
на чистой учётной записи для этой ревизии повторно не проверялись.
