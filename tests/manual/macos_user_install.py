#!/usr/bin/env python3
"""Test stable user App path and installer refresh against real launchd.
Local ad-hoc arm64 fixtures, temporary HOME and file-key profiles only.
This does not test the universal release, installer downloads, Gatekeeper or Bluetooth.
Usage: macos_user_install.py /path/to/current/shum
"""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
source = Path(sys.argv[1]).resolve(strict=True)
assert sys.platform == "darwin"
version = subprocess.check_output([str(source), "--version"], text=True).strip().removeprefix("shum ")
with tempfile.TemporaryDirectory(prefix="shum-user-app-") as temporary:
    home = Path(temporary) / "user home"
    install = home / ".local/share/shum"
    install.mkdir(parents=True)
    (install / ".shum-installer").write_text("shum-install-v1\n")
    root = home / "profiles"
    ids = []
    environment = dict(os.environ, HOME=str(home), SHUM_DATA_DIR=str(root))
    stable = install / "Shum.app"
    command = stable / "Contents/MacOS/shum"
    for tag in ("a", "b"):
        app = install / "versions" / (tag * 64) / "Shum.app"
        (app / "Contents/MacOS").mkdir(parents=True)
        shutil.copy2(source, app / "Contents/MacOS/shum")
        template = (ROOT / "native/Info.plist").read_text().replace("__SHUM_VERSION__", version)
        info = plistlib.loads(template.encode())
        info["ShumLocalTestBuild"] = tag
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
        subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", "--identifier", "org.shum.cli", str(app)], check=True)
    stable.symlink_to("versions/" + "a" * 64 + "/Shum.app")

    def cli(*args):
        result = subprocess.run([str(command), "--json", *args], env=environment,
                                capture_output=True, text=True, timeout=90)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return json.loads(result.stdout)

    def endpoint(profile):
        return json.loads((root / profile / "daemon.json").read_text())

    try:
        assert cli("daemon", "--refresh")["activeProfilesRefreshed"] == 0
        assert not root.exists(), "first install refresh must not create a profile"
        created = cli("--no-bluetooth", "--relay", "ws://127.0.0.1:9", "--push-url", "off",
                      "init", "--headless", "--name", "Temporary user App")
        profile = created["profile"]["id"]
        ids.append(profile)
        cli("-p", profile, "daemon", "--install")
        before = cli("-p", profile, "status")
        pid = endpoint(profile)["pid"]
        agent = home / f"Library/LaunchAgents/org.shum.cli.{profile}.plist"
        info = plistlib.loads(agent.read_bytes())
        assert Path(info["ProgramArguments"][0]) == install.resolve() / "Shum.app/Contents/MacOS/shum"
        assert info["KeepAlive"] is False
        assert not (root / "services").exists()
        registry = (root / "profiles.json").read_bytes()
        stable.unlink()
        stable.symlink_to("versions/" + "b" * 64 + "/Shum.app")
        assert cli("daemon", "--refresh")["activeProfilesRefreshed"] == 1
        after = cli("-p", profile, "status")
        assert endpoint(profile)["pid"] != pid
        assert after["build"]["sha256"] != before["build"]["sha256"]
        assert after["build"]["sha256"] == hashlib.sha256(command.read_bytes()).hexdigest()
        assert before["card"] == after["card"] and before["messages"] == after["messages"]
        assert Path(plistlib.loads(agent.read_bytes())["ProgramArguments"][0]) == install.resolve() / "Shum.app/Contents/MacOS/shum"
        cli("daemon", "--uninstall")
        assert not agent.exists()
        result = subprocess.run(["/bin/launchctl", "print", f"gui/{os.getuid()}/org.shum.cli.{profile}"], capture_output=True)
        assert result.returncode != 0
        assert (root / "profiles.json").read_bytes() == registry
        assert (root / profile / "keys.bin").exists() and (root / profile / "messages.sqlite").exists()
        assert cli("daemon", "--refresh")["activeProfilesRefreshed"] == 0
        cli("daemon", "--uninstall")
        print("PASS real launchd: stable ~/.local App path, KeepAlive=false, bundle reused directly")
        print("PASS installer refresh changes daemon PID/hash, preserves identity/history, skips inactive profiles")
        print("PASS bootout, agent removal, idempotent uninstall, profile files retained")
        print("Temporary local ad-hoc arm64 fixtures; no universal, Gatekeeper, Bluetooth or release check")
    finally:
        if root.exists(): cli("daemon", "--uninstall")
