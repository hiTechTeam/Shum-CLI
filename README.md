# Shum CLI

English · [Русский](README.ru.md)

Terminal client for Shum, built on [Shum Core](https://github.com/hiTechTeam/Shum-Core). Chats, invitations, QR codes, profiles, encrypted history and a background service. Messages travel over Bluetooth or Nostr.

## Install

Current release: **0.1.7 preview, Homebrew revision 2 (0.1.7_2), macOS 15+, Apple Silicon and Intel**.

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
shum contacts
shum add 'shum://c4/<signed-profile>'
shum add --image ~/Desktop/invite.png
shum invite 'CONTACT_ID'
shum open 'CONTACT_ID'
shum send 'CONTACT_ID' "Hello"
shum status
```

Replace CONTACT_ID with an ID from `shum contacts`. Contact commands use Shum IDs, unique prefixes of at least eight characters, or full Nostr public keys. Names are for display. Full cards and QR images can be added offline; fetching a new card by network ID requires the peer to respond.

In an open chat, `/invite` invites that peer. `/qr` shows your own card. Use `/help` for commands; Ctrl+Q exits the interface. Unread counts and typing indicators appear in the chat list. Terminal uses cell colors for avatars; Warp uses PNG.

In an open chat, type `/react` and press Enter. Select a message with ↑/↓, press Enter, then choose a pixel reaction with the arrows and confirm with Enter. Esc goes back or cancels. Selecting your current reaction removes it. The menu and chat history use the same eight pixel icons as iOS. Under a message, matching reactions share one icon followed by both names separated by `/`; different reactions each show their icon and author. Warp uses compact PNG icons.

The service keeps running after the interface closes. `shum daemon --stop` stops it; `shum daemon --install` enables login startup. After an update, the next CLI command replaces an outdated service and preserves the outgoing queue. Allow Bluetooth access for Shum when macOS asks.

For local profiles, use the full ID from `shum profile list`: `shum profile use 'PROFILE_ID'`, `shum -p 'PROFILE_ID' chats`, or `shum profile delete 'PROFILE_ID'`. The deletion prompt shows the name and ID; the name confirms the operation and never selects its target.

Registration animates key assembly, smoothly fills a progress bar and checks off key generation, encryption, protected storage and verification before asking for a name. The first avatar is then derived from the public signing key, as on iPhone. Keys stay in memory until the avatar is confirmed; preparation uses temporary encrypted storage. Profiles may share a display name and have independent IDs, keys and history.

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

Profile data stays in `~/Library/Application Support/org.Shum.Shum` or your `--data-dir`; profile keys may be in Keychain.

To irreversibly delete all profiles, keys (including Keychain), history and service caches:

```sh
shum daemon --uninstall --purge
brew uninstall shum
```

Type `DELETE` when prompted. For scripts, add `--confirm DELETE`. Available since 0.1.7. Without `--data-dir`, it includes the default directory and other data directories registered in your Shum LaunchAgents. With `--data-dir`, it removes only that directory's Shum data. Unrelated files are preserved and reported. The package manager or installation script removes the installed application separately.

For 0.1.6, delete each profile with `shum profile delete 'PROFILE_ID'` before uninstalling services and the package. This removes its Keychain keys too. You can then delete the remaining data directory manually.

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

Package revision 2 adds pixel reactions and author names in chat history. See the release verification report for packaging checks. Bluetooth permission persistence and iPhone delivery after this revision still need real-device verification.

[Release verification](docs/release-verification-0.1.7-r1.md) · [Distribution](docs/distribution.md) · [Protocol status](docs/protocol-status.md) · [Detailed guide in Russian](docs/cli-guide.ru.md) · [Issues](https://github.com/hiTechTeam/Shum-CLI/issues)

## License

[MIT](LICENSE), copyright 2026 hiTechTeam.
