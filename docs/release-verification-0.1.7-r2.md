# Shum CLI 0.1.7, package revision 2

Verified on 10 October 2026. App version: `0.1.7`.
Homebrew package version: `0.1.7_2`. Release tag: `v0.1.7-r2`.

Source: `a5148025a12224bcdad0bae824e15e0de804a7d5`.
Tap: `d1b3222341b33bdab3c753230e17c20dd09631d7`.

## Checks

* Release test suite: 53 passed, 0 failed, 0 ignored. Includes local relay
  exchange and reaction history grouping, authors, scrolling, resizing and modal coverage.
* Clippy (`--lib --tests -- -D warnings`), formatting and diff checks passed.
* Package verifier passed: signed Shum.app, plist version, installer receipt,
  symlink, checksums, Homebrew revision and both architecture slices.
* Both CLI slices and embedded Bluetooth helpers require macOS 15.0.0.
  Architectures: arm64 and x86_64; no non-system dynamic dependencies.
* Rosetta `arch -x86_64 .../shum --version`: `shum 0.1.7`.
* Designated requirement matches revision 1 for both architectures:
  `designated => identifier "org.shum.cli" and certificate leaf = H"9ce51b5c3430742e7860fc77583b41baa8355085"`. No binary hash is part of the requirement.
* Disposable-profile upgrade from revision 1 to revision 2 passed.
  The binary hash changed while the version stayed 0.1.7, and the service PID
  changed automatically through the stable opt path. Deleting the old Cellar
  directory did not break startup. Profile identity and history were preserved.
  Uninstall removed the temporary LaunchAgent and retained the profile files.

Binary SHA-256: `bbbbd1ef471c8a29c800edb821d91d72801a51be0f0705ed429b60414dd3cd13`.
Archive SHA-256: `3cbcda7e93f0a5d1009d7e36da1098771f6771ce9b0ae5dbe0e2ab693933bc64`.

## Update

[CLI release](https://github.com/hiTechTeam/Shum-CLI/releases/tag/v0.1.7-r2)
· [Homebrew release](https://github.com/hiTechTeam/homebrew-shum/releases/tag/v0.1.7-r2).

```sh
brew update
brew upgrade shum
shum
```

Reopen the chat interface for the new rendering. Script installations update by
running `install.sh` again. The previous release archives remain available.
CI remains manual. The owner's installed app was not changed by these checks.

## Limits

Actual Intel hardware, clean-account Gatekeeper, Bluetooth permission persistence
and iPhone message exchange after this revision were not tested. Rosetta does
not replace an Intel hardware test. The app is self-signed, not notarized;
the pkg container is unsigned. Native-window visual inspection was not performed;
UI tests inspect rendered terminal cells and image protocol output.

## Русский

Версия приложения 0.1.7, пакет Homebrew 0.1.7_2. Прошли 53 теста,
clippy, проверка подписанного пакета и запуск Intel-среза через Rosetta.
Обновление службы по новому хэшу проверено на временном профиле:
служба заменена автоматически, история и идентификатор профиля сохранены.
Проверено удаление тестового LaunchAgent с сохранением данных.
Пользовательское приложение в ходе проверки не обновлялось.
Проверки Bluetooth и обмена с iPhone после этой ревизии пока не выполнены.
