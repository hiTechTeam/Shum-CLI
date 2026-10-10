# Shum CLI 0.1.8 verification

Verified on 10 October 2026. Source: `54aed50288e5ac09947d01cef25a16169b83f4f7`.
Tap: `f374b3fad3dd5dbbaa7afbf36a37bf48fba88b58`. Release tag: `v0.1.8`.

## Checks

* Release test suite on Apple Silicon: 64 passed, 0 failed, 0 ignored.
  Includes relay invitations, refusal and later acceptance, persistent invitation
  history, replay handling, chat clearing, reaction and language menus, profile
  images, message delivery, service replacement and uninstall.
* Clippy (all targets, warnings denied), formatting and diff checks passed.
* All ten catalogs contain 301 translations; keys, placeholders and controls checked.
* Shellcheck and POSIX syntax checks passed. Installer fixtures passed checksum
  and signature rejection, update rollback, repeat installation, Homebrew refusal,
  PATH guidance, Intel selection and removal preserving profile data.
* Package verifier passed: signature, both architectures, embedded Bluetooth
  helpers, plist version, package receipt, symlink and SHA-256 checksums.
* Both architectures require macOS 15.0.0 and only system dynamic libraries.
* Rosetta `arch -x86_64 .../shum --version`: `shum 0.1.8`.
* Signature requirement is identical to 0.1.7 revision 2, for both architectures:
  `designated => identifier "org.shum.cli" and certificate leaf = H"9ce51b5c3430742e7860fc77583b41baa8355085"`. No executable hash in the requirement.
* Disposable-profile upgrade from the signed 0.1.7 revision 2 archive to 0.1.8
  passed through a simulated Homebrew opt path. The service PID and binary hash
  changed automatically. Removing the old Cellar directory did not break startup.
  Profile identity, card and saved history were preserved.
* Uninstall removed the temporary LaunchAgent and service endpoint while
  preserving profile files. KeepAlive remained false.
* Homebrew formula Ruby syntax passed. Revision resets to zero for the new version.

Binary SHA-256: `1664127443d9b1542eefd9b018ae92908653b0503d20f66d14edcff43c25821e`.
Archive SHA-256: `9019f423e6a0edc1df7b3afbb5cb35496678770d377371a0ad4913a5f0ae8125`.

## Update

[CLI release](https://github.com/hiTechTeam/Shum-CLI/releases/tag/v0.1.8)
· [Homebrew release](https://github.com/hiTechTeam/homebrew-shum/releases/tag/v0.1.8).

```sh
brew update
brew upgrade shum
shum --version
```

Expected version: `shum 0.1.8`. Reopen the chat interface for the new controls.
The service updates on the next CLI command. Script installations update by
running `install.sh` again. Previous release archives are preserved. CI remains manual.

## Limits

The owner's installed application and real profiles were not updated by these checks.
The upgrade test used the Homebrew directory layout, not a real brew upgrade.
Bluetooth permission persistence, iPhone exchange after this release, a clean-account
Gatekeeper launch and actual Intel hardware were not tested. The Intel slice was
launched under Rosetta, which is not a substitute for Intel hardware.
The app is self-signed and not notarized; the pkg container is unsigned.
UI tests inspect terminal cells and graphics output, not native-window screenshots.

## Русский

Прошли 64 теста, clippy без замечаний, проверка 301 перевода в каждом из десяти
языков, shellcheck и проверки установщика. Подпись пакета, обе архитектуры,
минимальная macOS и контрольные суммы проверены. Intel-срез запущен под Rosetta.

Обновление 0.1.7 → 0.1.8 проверено на временном профиле со стабильным путём opt:
служба заменяется автоматически, идентификатор профиля и история сохраняются.
Удаление тестовой службы сохраняет данные и удаляет LaunchAgent.
Пользовательская установка и настоящие профили в проверках не изменялись.

Настоящий brew upgrade, сохранение разрешения Bluetooth, обмен с iPhone после
этого выпуска, Gatekeeper на чистой учётной записи и настоящий Intel Mac
не проверялись. Автоматический CI остаётся выключенным.
