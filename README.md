# Shum CLI

English · [Русский](README.ru.md)

Terminal client for Shum, built on [Shum Core](https://github.com/hiTechTeam/Shum-Core). Chats, invitations, QR codes, profiles, encrypted history and a background service. Messages travel over Bluetooth or Nostr.

## Install

Current release: **0.1.8 preview (Homebrew 0.1.8_1), macOS 15+, Apple Silicon and Intel**.

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

In an open chat, `/invite` invites that peer. The invitation and reply appear in the chat history. The recipient uses `/accept` or `/decline`, and can still use `/accept` after declining. `/qr` shows your own card. Use `/help` for commands; Ctrl+Q exits the interface. Unread counts and typing indicators appear in the chat list. Terminal uses cell colors for avatars; Warp uses PNG.

In an open chat, press **Ctrl+R**, or type `/react` and press Enter. Select a message with ↑/↓, press Enter, then choose a pixel reaction with the arrows and confirm with Enter. Esc goes back or cancels. Selecting your current reaction removes it. The menu and chat history use the same eight pixel icons as iOS. Under a message, matching reactions share one icon followed by both names separated by `/`; different reactions each show their icon and author. Warp uses compact PNG icons. Avatars and reactions outside the picker remain visible. Opening the picker with Ctrl+R preserves the draft.

The service keeps running after the interface closes. `shum daemon --stop` stops it; `shum daemon --install` enables login startup. After an update, the next CLI command replaces an outdated service and preserves the outgoing queue. Allow Bluetooth access for Shum when macOS asks.

For local profiles, use the full ID from `shum profile list`: `shum profile use 'PROFILE_ID'`, `shum -p 'PROFILE_ID' chats`, or `shum profile delete 'PROFILE_ID'`. The deletion prompt shows the name and ID; the name confirms the operation and never selects its target.

Registration animates key assembly, smoothly fills a progress bar and checks off key generation, encryption, protected storage and verification before asking for a name. The first avatar is then derived from the public signing key, as on iPhone. Keys stay in memory until the avatar is confirmed; preparation uses temporary encrypted storage. Profiles may share a display name and have independent IDs, keys and history.

## Language

The CLI supports the same ten languages as iOS: Russian, English, Spanish, Simplified Chinese, Hindi, French, Japanese, Brazilian Portuguese, Arabic and Korean.

```sh
shum language en        # save a language for this data directory
shum language system    # follow the system again
shum --lang ru          # choose for this invocation only
shum language           # show the current language and available codes
```

In a chat, `/language en` changes and saves the language immediately. `/language` or Ctrl+L opens the language menu. Choose with ↑/↓, confirm with Enter or cancel with Esc. Avatars and reaction icons outside the menu remain visible. Codes: `ru`, `en`, `es`, `zh-Hans`, `hi`, `fr`, `ja`, `pt-BR`, `ar`, `ko`, `system`.

Priority: `--lang`, `SHUM_LANG`, saved setting, system locale. The system locale uses `LC_ALL`, then `LC_MESSAGES`, then `LANG`; if none are set, macOS preferred languages are used. Unsupported locales and `C`/`POSIX` use English. Chinese locales map to Simplified Chinese; Portuguese locales use Brazilian Portuguese. Settings are stored in `cli-settings.json` in the data directory, without creating a profile. Commands, IDs, JSON field names and your messages stay unchanged. System and dependency diagnostics may remain in their original language. Arabic text shaping depends on the terminal; the interface keeps its column order.

### Windows (preview 0.1.3)

Shum for Windows has its own version line, starting at 0.1.0. Windows 10 and 11, x64 and ARM64. In PowerShell, install or update with:

```powershell
irm https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.ps1 | iex
```

The script picks the build for your processor, checks SHA-256, installs `shum.exe` in `%LOCALAPPDATA%\Programs\Shum` and adds it to your user PATH. Builds are cross-compiled on macOS (`scripts/package_windows.sh`) and are not code-signed. They have not been run on Windows hardware yet. Bluetooth only discovers nearby devices so far; other devices cannot find a Windows computer, and computers without Bluetooth chat over the internet.

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

For Windows:

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/hiTechTeam/Shum-CLI/main/install.ps1))) -Uninstall
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
python3 scripts/check_locales.py
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

CLI and Core are separate repositories and releases. Core dependencies pin one Git revision. CI runs manually. The current client implements the v1 draft; multi-device sync is planned. Linux and Windows packages are pending.

Version 0.1.8 adds ten languages, keyboard menus and invitation history, and fixes images around popup menus. See the [release verification report](docs/release-verification-0.1.8-r1.md). Bluetooth permission persistence and iPhone delivery after this release still need real-device verification.

[Release verification](docs/release-verification-0.1.8-r1.md) · [Distribution](docs/distribution.md) · [Protocol status](docs/protocol-status.md) · [Detailed guide in Russian](docs/cli-guide.ru.md) · [Issues](https://github.com/hiTechTeam/Shum-CLI/issues)

## License

[MIT](LICENSE), copyright 2026 hiTechTeam.
