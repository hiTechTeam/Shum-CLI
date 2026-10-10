# Shum CLI 0.1.7, package revision 1

Verified on 10 October 2026. Application version: `0.1.7`.
Homebrew package version: `0.1.7_1`. Release tag: `v0.1.7-r1`.

Source: `2dd2520fe46925f6a39047d2963a9bede95b8997`.
Tap: `7e46b64c9a4f8d78608c8a67be23969c01036848`.

## Checks

* Release test suite: 48 passed, 0 failed, 0 ignored.
  Includes message/reaction selection, live snapshot changes, cancellation,
  deleted messages, existing reaction toggles, Warp profile images and local relay exchange.
* Clippy with `--all-targets -- -D warnings`, formatting and diff checks passed.
* All eight 12×12 reaction grids match the Swift artwork.
* Package verifier passed: signed Shum.app, archive and installer checksums,
  versioned plist, symlink, receipt and Homebrew revision.
* `lipo -info`: x86_64 and arm64. Both CLI slices and embedded Bluetooth helpers
  require macOS 15.0.0. No non-system dynamic dependencies.
* `arch -x86_64 .../shum --version`: `shum 0.1.7` under Rosetta.
* Designated requirement matches the previous package for both architectures:
  `identifier "org.shum.cli" and certificate leaf = H"9ce51b5c3430742e7860fc77583b41baa8355085"`.
  No binary hash is part of that requirement.
* Disposable-profile upgrade from the original 0.1.7 archive to revision 1 passed.
  The changed binary hash replaced the service PID automatically at a stable opt path,
  including after deletion of the old Cellar directory. Profile identity, card and
  history were preserved. Uninstall removed the test LaunchAgent and kept profile files.
* All five assets in each GitHub release match local SHA-256 digests.
  Both repositories resolve `latest` to `v0.1.7-r1`.
* Homebrew sees stable version `0.1.7`, revision `1`, and reports the installed
  original 0.1.7 as outdated. The owner's application was not upgraded during this check.

Binary SHA-256: `502cc33524e0288c78a40e4fd845e27d555768415062340c2df5054493cf9934`.
Archive SHA-256: `dde3ee0e637e0ebadb9d6259e6aed5eae50f303511073b9fe3d85f1117802207`.

## Publication and update

[CLI release](https://github.com/hiTechTeam/Shum-CLI/releases/tag/v0.1.7-r1)
· [Homebrew release](https://github.com/hiTechTeam/homebrew-shum/releases/tag/v0.1.7-r1).
Original v0.1.7 tags and assets are preserved. CI remains manual.

```sh
brew update
brew upgrade shum
shum
```

Reopen an existing chat interface to load its new controls.
Script installations update by running `install.sh` again.

## Limits

Actual Intel hardware, clean-account Gatekeeper and Bluetooth permission persistence
for this revision were not tested. Rosetta does not replace an Intel hardware test.
The app is self-signed, not notarized; the installer container is unsigned.
Owner-profile message exchange after revision 1 still needs a real-device check.

## Русский

Версия приложения осталась 0.1.7, ревизия Homebrew: 0.1.7_1.
Прошли 48 тестов, проверки подписи и обеих архитектур, а также обновление
службы по изменившемуся хэшу на временном профиле с сохранением данных.
GitHub latest и формула указывают на новую сборку; Homebrew распознаёт обновление.
Установленное приложение владельца в ходе проверки не заменялось.
Проверка Bluetooth и обмена с iPhone после этой ревизии пока не выполнена.
