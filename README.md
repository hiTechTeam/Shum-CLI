# Shum CLI

English · [Русский](README.ru.md)

Terminal client for Shum, built on [Shum Core](https://github.com/hiTechTeam/Shum-Core). Chats, invitations, QR codes, profiles, encrypted history and a background service. Messages travel over Bluetooth or Nostr.

## Install

Current release: **0.1.6 preview, macOS 15+, Apple Silicon and Intel**.

```sh
brew install hitechteam/shum/shum
shum
```

The first launch creates a profile. To update:

```sh
brew update
brew upgrade shum
```

Without Homebrew, use the same command to install or update:

```sh
curl -fsSL https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh | sh
```

The script checks SHA-256 and the app signature, installs in `~/.local/share/shum`, and links `~/.local/bin/shum`. It prints a PATH hint when needed and leaves shell files alone. It refuses to install alongside Homebrew.

[Downloads](https://github.com/hiTechTeam/Shum-CLI/releases) include a universal archive and a macOS installer. The app uses a self-signed certificate, not Apple Developer ID, and is not notarized. Gatekeeper on a clean account and a real Intel Mac remain untested.

## Use

```sh
shum chats
shum profile
shum qr
shum add 'shum://c4/<signed-profile>'
shum add --image ~/Desktop/invite.png
shum invite <contact-id>
shum open <contact-id>
shum send <contact-id> "Hello"
shum status
```

Contact commands use Shum IDs, unique prefixes of at least eight characters, or full Nostr public keys. Names are for display. Full cards and QR images can be added offline; fetching a new card by network ID requires the peer to respond.

In an open chat, `/invite` invites that peer. `/qr` shows your own card. Use `/help` for commands; Ctrl+Q exits the interface. Unread counts and typing indicators appear in the chat list. Terminal uses cell colors for avatars; Warp uses PNG.

The service keeps running after the interface closes. `shum daemon --stop` stops it; `shum daemon --install` enables login startup. After an update, the next CLI command replaces an outdated service and preserves the outgoing queue. Allow Bluetooth access for Shum when macOS asks.

## Uninstall

For Homebrew:

```sh
shum daemon --uninstall
brew uninstall shum
```

For the script installation:

```sh
curl -fsSL https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.sh | sh -s -- --uninstall
```

Profile data stays in `~/Library/Application Support/org.Shum.Shum` or your `--data-dir`; profile keys may be in Keychain. To erase everything, stop services, delete your data directory manually, and remove the profile's Shum items from Keychain.

If the binary is already gone, unload each Shum LaunchAgent, then delete its plist:

```sh
launchctl bootout "gui/$(id -u)" "$HOME/Library/LaunchAgents/org.shum.cli.<profile-id>.plist"
rm "$HOME/Library/LaunchAgents/org.shum.cli.<profile-id>.plist"
```

Replace the placeholder with the ID in the filename. `KeepAlive=false` prevents a restart loop when the executable is missing.

## Development and status

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

CLI and Core are separate repositories and releases. Core dependencies pin one Git revision. CI runs manually. The current client implements the v1 draft; multi-device sync is planned. Linux and Windows packages are pending.

The 0.1.6 release passed 31 CLI tests and Rosetta messaging checks. Homebrew upgrade, automatic service replacement and messages in both directions with an iPhone were verified. Bluetooth permission was requested again during the transition from ad hoc to certificate signing; the next certificate-to-certificate upgrade remains to be checked.

[Release verification](docs/release-verification.md) · [Distribution](docs/distribution.md) · [Protocol status](docs/protocol-status.md) · [Detailed guide in Russian](docs/cli-guide.ru.md) · [Issues](https://github.com/hiTechTeam/Shum-CLI/issues)

## License

[MIT](LICENSE), copyright 2026 hiTechTeam.
