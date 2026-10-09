"""Verify graceful CLI service upgrade with a disposable offline profile.

Usage: service_upgrade.py OLD_SHUM NEW_SHUM
OLD_SHUM can have the same version but must have a different binary hash. No Bluetooth or external servers.
"""
import json
import pathlib
import subprocess
import sys
import tempfile
import time

old, new = [str(pathlib.Path(p).resolve()) for p in sys.argv[1:3]]
with tempfile.TemporaryDirectory(prefix="shum-service-upgrade-") as directory:
    root = pathlib.Path(directory) / "profiles"

    def cli(binary, *args):
        result = subprocess.run([binary, "--data-dir", str(root), "--json", *args],
                                capture_output=True, text=True, timeout=35)
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return json.loads(result.stdout)

    created = cli(old, "--no-bluetooth", "--relay", "ws://127.0.0.1:9",
                  "--push-url", "off", "init", "--headless", "--name", "Upgrade test")
    profile = created["profile"]["id"]
    previous = subprocess.Popen([old, "--data-dir", str(root), "-p", profile,
                                 "daemon", "--run"], stdin=subprocess.DEVNULL,
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        endpoint = root / profile / "daemon.json"
        deadline = time.monotonic() + 8
        while not endpoint.exists():
            assert previous.poll() is None, "old service exited"
            assert time.monotonic() < deadline, "old service did not start"
            time.sleep(.05)
        before = cli(old, "-p", profile, "status")
        previous_build = before.get("build")
        after = cli(new, "-p", profile, "status")
        assert after["clientCommandVersion"] == 2
        assert after["build"]["version"]
        assert len(after["build"]["sha256"]) == 64
        assert after["build"] != previous_build
        assert previous.wait(timeout=5) == 0, "old service must checkpoint and exit gracefully"
        assert after["profile"]["ownerId"] == created["profile"]["ownerId"]
        for key in ["noiseKey", "signingKey", "nostrKey", "name", "avatarSeed"]:
            assert after["card"][key] == before["card"][key], key
        assert after["messages"] == before["messages"]
        assert len(cli(new, "profile", "list")["profiles"]) == 1
        cli(new, "-p", profile, "daemon", "--stop")
        print("PASS automatic service upgrade: graceful exit, stable identity and saved profile")
    finally:
        try:
            cli(new, "-p", profile, "daemon", "--stop")
        except Exception:
            pass
        if previous.poll() is None:
            previous.kill()
            previous.wait()
