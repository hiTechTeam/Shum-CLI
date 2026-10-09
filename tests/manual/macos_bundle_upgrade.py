#!/usr/bin/env python3
"""Replace two signed bundles through a disposable Homebrew-style opt path.
This tests layout and cleanup; it does not run brew or Bluetooth.
Usage: macos_bundle_upgrade.py OLD_ARCHIVE NEW_ARCHIVE
"""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import socket
import struct
import subprocess
import sys
import tarfile
import tempfile

assert sys.platform == "darwin"
archives = [Path(p).resolve(strict=True) for p in sys.argv[1:3]]
with tempfile.TemporaryDirectory(prefix="shum-opt-upgrade-") as temporary:
    prefix = Path(temporary) / "brew"
    root = Path(temporary) / "profiles"
    old = prefix / "Cellar/shum/build-A"
    new = prefix / "Cellar/shum/build-B"
    for archive, destination in zip(archives, [old, new]):
        destination.mkdir(parents=True)
        with tarfile.open(archive) as bundle:
            # Generated local packages contain only the verified Shum.app tree.
            assert all(m.name == "Shum.app" or m.name.startswith("Shum.app/") for m in bundle)
            bundle.extractall(destination)
    opt = prefix / "opt/shum"
    opt.parent.mkdir(parents=True)
    opt.symlink_to(old)
    binary = opt / "Shum.app/Contents/MacOS/shum"

    def cli(*args):
        result = subprocess.run([str(binary), "--data-dir", str(root), "--json", *args],
                                capture_output=True, text=True, timeout=90)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return json.loads(result.stdout)

    def endpoint(profile):
        return json.loads((root / profile / "daemon.json").read_text())

    def live_snapshot(profile):
        info = endpoint(profile)
        def exact(stream, count):
            data = b""
            while len(data) < count:
                chunk = stream.recv(count - len(data))
                assert chunk
                data += chunk
            return data
        with socket.create_connection(("127.0.0.1", info["port"]), timeout=5) as stream:
            request = b'{"command":"snapshot"}'
            stream.sendall(bytes.fromhex(info["token"]) + struct.pack(">I", len(request)) + request)
            size, = struct.unpack(">I", exact(stream, 4))
            return json.loads(exact(stream, size))["ok"]

    try:
        created = cli("--no-bluetooth", "--relay", "ws://127.0.0.1:9", "--push-url", "off",
                      "init", "--headless", "--name", "Signed upgrade")
        profile = created["profile"]["id"]
        cli("-p", profile, "daemon", "--install")
        before = cli("-p", profile, "status")
        previous_pid = endpoint(profile)["pid"]
        old_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        assert before["build"]["sha256"] == old_hash
        agent = Path.home() / f"Library/LaunchAgents/org.shum.cli.{profile}.plist"
        assert Path(plistlib.loads(agent.read_bytes())["ProgramArguments"][0]) == prefix.resolve() / "opt/shum/Shum.app/Contents/MacOS/shum"
        opt.unlink()
        opt.symlink_to(new)
        shutil.rmtree(old)  # Simulate brew cleanup while the old image is still loaded.
        assert live_snapshot(profile)["build"]["sha256"] == old_hash
        after = cli("-p", profile, "status")
        assert after["build"]["sha256"] == hashlib.sha256(binary.read_bytes()).hexdigest()
        assert after["build"]["sha256"] != old_hash
        assert endpoint(profile)["pid"] != previous_pid
        assert after["profile"]["ownerId"] == before["profile"]["ownerId"]
        assert after["card"] == before["card"]
        assert after["messages"] == before["messages"]
        info = plistlib.loads(agent.read_bytes())
        assert Path(info["ProgramArguments"][0]) == prefix.resolve() / "opt/shum/Shum.app/Contents/MacOS/shum" and info["KeepAlive"] is False
        assert not (root / "services").exists()
        cli("daemon", "--uninstall")
        assert not agent.exists() and not (root / profile / "daemon.json").exists()
        assert (root / profile / "messages.sqlite").exists() and (root / profile / "keys.bin").exists()
        print("PASS same certificate: direct bundle reuse, stable opt path, cleanup of loaded Cellar version")
        print("PASS changed hash replaces PID automatically; profile identity and history preserved")
        print("PASS uninstall removes temporary LaunchAgent and preserves profile files")
        print("Homebrew itself and Bluetooth were not exercised")
    finally:
        if binary.exists() and root.exists():
            cli("daemon", "--uninstall")
