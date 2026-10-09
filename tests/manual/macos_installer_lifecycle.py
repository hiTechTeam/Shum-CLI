#!/usr/bin/env python3
"""Install, upgrade and remove actual signed universal packages in a disposable HOME.
Only release downloads, brew command and the two fixed Homebrew probe paths are
isolated. codesign, tar, hashes, CLI and launchd are real. Not a clean macOS account.
Usage: macos_installer_lifecycle.py OLD_RELEASE_DIR NEW_RELEASE_DIR
"""
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import sqlite3
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
releases = [Path(p).resolve(strict=True) for p in sys.argv[1:3]]
assert sys.platform == "darwin" and len(releases) == 2
asset = "shum-macos-universal.tar.gz"
with tempfile.TemporaryDirectory(prefix="shum-real-installer-") as temporary:
    fixture = Path(temporary)
    home = fixture / "new user with spaces"
    home.mkdir()
    tools = fixture / "tools"
    tools.mkdir()
    script = fixture / "install.sh"
    script.write_text((ROOT / "install.sh").read_text()
                      .replace("/opt/homebrew/opt/shum", str(fixture / "brew-arm/opt/shum"))
                      .replace("/usr/local/opt/shum", str(fixture / "brew-intel/opt/shum")))
    (tools / "brew").write_text("#!/bin/sh\nexit 1\n")
    (tools / "brew").chmod(0o755)
    (tools / "curl").write_text('''#!/bin/sh
set -eu
case "$5" in
 https://github.com/hiTechTeam/homebrew-shum/releases/latest/download/*) ;;
 *) exit 1 ;;
esac
[ "$6" = -o ]
name=${5##*/}
cp "$SHUM_FIXTURE_RELEASE/$name" "$7"
''')
    (tools / "curl").chmod(0o755)
    profile_root = home / "Library/Application Support/org.Shum.Shum"
    environment = dict(os.environ, HOME=str(home), PATH=str(tools) + ":/usr/bin:/bin:/usr/sbin:/sbin",
                       TMPDIR=str(fixture), SHUM_FIXTURE_RELEASE=str(releases[0]))
    # Ensure inherited user configuration cannot select real profiles.
    environment.pop("SHUM_DATA_DIR", None)
    binary = home / ".local/bin/shum"
    install_dir = home / ".local/share/shum"
    ids = []

    def installer(*args, success=True):
        result = subprocess.run(["/bin/sh", str(script), *args], env=environment,
                                capture_output=True, text=True, timeout=120)
        assert (result.returncode == 0) == success, (result.stdout, result.stderr)
        return result.stdout + result.stderr

    def cli(*args):
        result = subprocess.run([str(binary), "--json", *args], env=environment,
                                capture_output=True, text=True, timeout=90)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return json.loads(result.stdout)

    def records(profile, stopped=False):
        database = profile_root / profile / "messages.sqlite"
        assert database.is_file(), ("profile database missing", database)
        if stopped:
            wal = database.with_name(database.name + "-wal")
            assert not wal.exists() or wal.stat().st_size == 0, "graceful stop must checkpoint WAL"
        uri = database.as_uri() + ("?mode=ro&immutable=1" if stopped else "?mode=ro")
        with closing(sqlite3.connect(uri, uri=True)) as db:
            return db.execute("SELECT bucket,id,position,payload FROM records ORDER BY bucket,id").fetchall()

    try:
        # Refuse a real archive with an incorrect SHA before publishing any files.
        bad = fixture / "bad release"
        bad.mkdir()
        shutil.copy2(releases[0] / asset, bad / asset)
        (bad / (asset + ".sha256")).write_text("0" * 64 + "  " + asset + "\n")
        environment["SHUM_FIXTURE_RELEASE"] = str(bad)
        assert "SHA-256 не совпадает" in installer(success=False)
        assert not binary.exists() and not install_dir.exists()
        environment["SHUM_FIXTURE_RELEASE"] = str(releases[0])
        output = installer()
        assert 'export PATH="$HOME/.local/bin:$PATH"' in output
        assert not profile_root.exists(), "installer must not create a profile"
        old_version = subprocess.check_output([str(binary), "--version"], env=environment, text=True).strip()
        old_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        old_path = binary.resolve()
        subprocess.run(["/usr/bin/arch", "-x86_64", str(binary), "--version"], env=environment, check=True)
        snapshots = {}
        pids = {}
        for name in ("Installer Alice", "Installer Bob"):
            created = cli("--no-bluetooth", "--relay", "ws://127.0.0.1:9", "--push-url", "off",
                          "init", "--headless", "--name", name)
            profile = created["profile"]["id"]
            ids.append(profile)
            cli("-p", profile, "daemon", "--install")
            snapshots[profile] = cli("-p", profile, "status")
            pids[profile] = json.loads((profile_root / profile / "daemon.json").read_text())["pid"]
            agent = home / f"Library/LaunchAgents/org.shum.cli.{profile}.plist"
            info = plistlib.loads(agent.read_bytes())
            assert Path(info["ProgramArguments"][0]) == install_dir.resolve() / "Shum.app/Contents/MacOS/shum", info["ProgramArguments"]
            assert info["KeepAlive"] is False
        registry = (profile_root / "profiles.json").read_bytes()
        # Idempotent install leaves matching running services intact.
        installer()
        for profile in ids:
            assert json.loads((profile_root / profile / "daemon.json").read_text())["pid"] == pids[profile]
        environment["SHUM_FIXTURE_RELEASE"] = str(releases[1])
        installer()
        new_version = subprocess.check_output([str(binary), "--version"], env=environment, text=True).strip()
        assert old_version != new_version, "need two distinct local build versions"
        assert old_hash != hashlib.sha256(binary.read_bytes()).hexdigest()
        assert old_path.exists(), "keep the previous signed App intact"
        retained = {}
        for profile in ids:
            after = cli("-p", profile, "status")
            assert after["build"]["version"] == new_version.removeprefix("shum ")
            assert after["build"]["sha256"] == hashlib.sha256(binary.read_bytes()).hexdigest()
            assert json.loads((profile_root / profile / "daemon.json").read_text())["pid"] != pids[profile]
            assert snapshots[profile]["card"] == after["card"]
            assert snapshots[profile]["messages"] == after["messages"]
            retained[profile] = records(profile)
        # Bad update preserves both stable command and running daemon.
        environment["SHUM_FIXTURE_RELEASE"] = str(bad)
        assert "SHA-256 не совпадает" in installer(success=False)
        assert subprocess.check_output([str(binary), "--version"], env=environment, text=True).strip() == new_version
        assert (profile_root / "profiles.json").read_bytes() == registry
        installer("--uninstall")
        assert not binary.exists() and not binary.is_symlink() and not install_dir.exists()
        assert (profile_root / "profiles.json").read_bytes() == registry
        for profile in ids:
            assert not (home / f"Library/LaunchAgents/org.shum.cli.{profile}.plist").exists()
            assert not (profile_root / profile / "daemon.json").exists()
            result = subprocess.run(["/bin/launchctl", "print", f"gui/{os.getuid()}/org.shum.cli.{profile}"], capture_output=True)
            assert result.returncode != 0
            assert records(profile, stopped=True) == retained[profile]
            assert (profile_root / profile / "keys.bin").exists()
        installer("--uninstall")
        assert not (home / ".zshrc").exists() and not (home / ".zprofile").exists()
        print(f"PASS actual signed universal App: {old_version} -> {new_version}, SHA checks, Rosetta --version")
        print("PASS repeat install keeps PID; upgrade refreshes both real LaunchAgents; identity/history preserved")
        print("PASS incorrect SHA aborts first install/update; full --uninstall removes App/bin/agents and preserves DB/keys")
        print("Local release fixtures, existing macOS account with temporary HOME: Gatekeeper/Bluetooth/real releases not checked")
    finally:
        if binary.exists(): installer("--uninstall")
