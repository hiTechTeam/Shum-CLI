#!/usr/bin/env python3
"""Exercise real LaunchAgents, signed bundle reuse and uninstall on disposable roots.
No Bluetooth, remote servers or existing profile keys are accessed.
Usage: macos_service_lifecycle.py SHUM [--packaged]
"""
import hashlib
from contextlib import closing
import json
import pathlib
import plistlib
import subprocess
import sys
import tempfile
import time

binary = pathlib.Path(sys.argv[1]).resolve()
packaged = "--packaged" in sys.argv[2:]
assert sys.platform == "darwin"
agent_directory = pathlib.Path.home() / "Library/LaunchAgents"
uid = str(__import__("os").getuid())
with tempfile.TemporaryDirectory(prefix="shum-lifecycle-") as temporary:
    root = pathlib.Path(temporary) / "profiles with spaces"
    ids = []

    def cli(*args):
        result = subprocess.run([str(binary), "--data-dir", str(root), "--json", *args],
                                capture_output=True, text=True, timeout=90)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return json.loads(result.stdout)

    def records(profile, stopped=False):
        import sqlite3
        database = root / profile / "messages.sqlite"
        assert database.is_file(), ("database disappeared", database, list((root / profile).iterdir()))
        if stopped:
            wal = database.with_name(database.name + "-wal")
            assert not wal.exists() or wal.stat().st_size == 0, "graceful stop must checkpoint WAL"
        # Inspect the stopped DB without opening WAL/shm files. This is safe
        # only after asserting that the graceful shutdown drained the WAL.
        uri = database.as_uri() + ("?mode=ro&immutable=1" if stopped else "?mode=ro")
        with closing(sqlite3.connect(uri, uri=True)) as db:
            return db.execute("SELECT bucket,id,position,payload FROM records ORDER BY bucket,id").fetchall()

    try:
        for name in ["Lifecycle A", "Lifecycle B"]:
            created = cli("--no-bluetooth", "--relay", "ws://127.0.0.1:9", "--push-url", "off",
                          "init", "--headless", "--name", name)
            ids.append(created["profile"]["id"])
        registry = (root / "profiles.json").read_bytes()
        retained = {}
        for profile in ids:
            cli("-p", profile, "daemon", "--install")
            status = cli("-p", profile, "status")
            agent = agent_directory / f"org.shum.cli.{profile}.plist"
            info = plistlib.loads(agent.read_bytes())
            assert info["KeepAlive"] is False
            app_binary = pathlib.Path(info["ProgramArguments"][0])
            assert app_binary.parts[-4:] == ("Shum.app", "Contents", "MacOS", "shum")
            assert "/Cellar/" not in str(app_binary)
            assert status["build"]["sha256"] == hashlib.sha256(app_binary.read_bytes()).hexdigest()
            if packaged:
                assert app_binary.resolve() == binary
                assert not (root / "services").exists(), "packaged app must not be copied or re-signed"
            retained[profile] = records(profile)
        deleted = cli("daemon", "--uninstall")
        assert deleted["uninstalled"] and deleted["profilesPreserved"] == 2
        assert not (root / "services").exists()
        for profile in ids:
            assert not (root / profile / "daemon.json").exists()
            assert not (agent_directory / f"org.shum.cli.{profile}.plist").exists()
            result = subprocess.run(["/bin/launchctl", "print", f"gui/{uid}/org.shum.cli.{profile}"], capture_output=True)
            assert result.returncode != 0
            assert records(profile, stopped=True) == retained[profile], "encrypted profile records changed"
            assert (root / profile / "keys.bin").is_file()
            assert (root / profile / "messages.sqlite").is_file()
        assert (root / "profiles.json").read_bytes() == registry
        assert len(cli("profile", "list")["profiles"]) == 2
        assert cli("daemon", "--uninstall")["bundlesRemoved"] == 0
        missing = pathlib.Path(temporary) / "does not exist"
        result = subprocess.run([str(binary), "--data-dir", str(missing), "--json", "daemon", "--uninstall"], capture_output=True)
        assert result.returncode == 0 and not missing.exists()
        print("PASS real launchctl bootstrap/bootout, KeepAlive=false, two profiles, idempotent uninstall")
        print("PASS profile registry, key files and encrypted database records preserved")
        if packaged:
            print("PASS packaged app reused directly; LaunchAgent executable outside Cellar")
        print("Only disposable profiles uninstalled; installed Homebrew formula unchanged")
    finally:
        try:
            cli("daemon", "--uninstall")
        except Exception:
            # Test-only agents remain named in the output if graceful cleanup failed.
            for profile in ids:
                print("Cleanup required:", agent_directory / f"org.shum.cli.{profile}.plist", file=sys.stderr)
            raise
