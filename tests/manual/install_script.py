#!/usr/bin/env python3
"""Exercise POSIX installer failure/upgrade/removal paths without network or real profiles.
Only the two fixed Homebrew probe paths are redirected into the disposable fixture.
Downloads and macOS inspection tools are replaced with local test doubles.
"""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="shum-installer-test-") as temporary:
    root = Path(temporary)
    tools = root / "tools"
    tools.mkdir()
    home = root / "home with spaces"
    home.mkdir()
    source = (ROOT / "install.sh").read_text().replace("/opt/homebrew/opt/shum", str(root / "brew-arm/opt/shum")).replace("/usr/local/opt/shum", str(root / "brew-intel/opt/shum"))
    script = root / "install.sh"
    script.write_text(source)
    script.chmod(0o755)

    def tool(name, body):
        p = tools / name
        p.write_text("#!/bin/sh\nset -eu\n" + body + "\n")
        p.chmod(0o755)

    tool("uname", 'case "$1" in -s) echo "${FIXTURE_OS:-Darwin}" ;; -m) echo "${FIXTURE_ARCH:-arm64}" ;; esac')
    tool("sw_vers", 'echo 15.6.1')
    tool("brew", 'test "${FIXTURE_BREW:-no}" = yes')
    tool("codesign", 'test "${FIXTURE_SIGNATURE:-valid}" = valid')
    tool("plutil", 'cat "$FIXTURE_RELEASE/version"')
    tool("curl", '''case "$5" in
 https://github.com/hiTechTeam/homebrew-shum/releases/latest/download/*) ;;
 *) echo "Unexpected release URL: $*" >&2; exit 1 ;;
 esac
 name=${5##*/}
 [ "$6" = -o ]
 cp "$FIXTURE_RELEASE/$name" "$7"''')
    release = root / "release"
    release.mkdir()
    asset = "shum-macos-universal.tar.gz"

    def publish(version, bad=False):
        app = root / "stage/Shum.app"
        if app.parent.exists(): shutil.rmtree(app.parent)
        (app / "Contents/MacOS").mkdir(parents=True)
        (app / "Contents/_CodeSignature").mkdir()
        (app / "Contents/Info.plist").write_text("fixture info")
        (app / "Contents/_CodeSignature/CodeResources").write_text("fixture signature")
        binary = app / "Contents/MacOS/shum"
        binary.write_text(f'''#!/bin/sh
case "$*" in
 --version) echo 'shum {version}' ;;
 'daemon --refresh') printf '%s\\n' 'refresh {version}' >> "$HOME/calls" ;;
 'daemon --uninstall') printf '%s\\n' uninstall >> "$HOME/calls" ;;
 *) exit 1 ;;
esac
''')
        binary.chmod(0o755)
        with tarfile.open(release / asset, "w:gz") as archive:
            archive.add(app, arcname="Shum.app")
        digest = "0" * 64 if bad else hashlib.sha256((release / asset).read_bytes()).hexdigest()
        (release / (asset + ".sha256")).write_text(f"{digest}  {asset}\n")
        (release / "version").write_text(version)
        return digest

    def run(*args, success=True, **extra):
        env = dict(os.environ, HOME=str(home), PATH=str(tools) + ":/usr/bin:/bin:/usr/sbin:/sbin",
                   FIXTURE_RELEASE=str(release), TMPDIR=str(root), **extra)
        result = subprocess.run(["/bin/sh", str(script), *args], env=env, capture_output=True, text=True)
        assert (result.returncode == 0) == success, (result.stdout, result.stderr)
        return result.stdout + result.stderr

    bad_output = run(success=False, FIXTURE_OS="Linux")
    assert "macOS 15+" in bad_output and "github.com/hiTechTeam/Shum-CLI" in bad_output
    assert not (home / ".local").exists()
    assert "Homebrew" in run(success=False, FIXTURE_BREW="yes")
    assert not (home / ".local/share/shum").exists()
    publish("1.0.0", bad=True)
    assert "SHA-256 не совпадает" in run(success=False)
    assert not (home / ".local/bin/shum").exists()
    publish("1.0.0")
    assert "Подпись" in run(success=False, FIXTURE_SIGNATURE="bad")
    assert not (home / ".local/share/shum").exists()
    first = publish("1.0.0")
    output = run()
    assert 'export PATH="$HOME/.local/bin:$PATH"' in output
    command = home / ".local/bin/shum"
    assert subprocess.check_output([str(command), "--version"], text=True).strip() == "shum 1.0.0"
    old = home / f".local/share/shum/versions/{first}/Shum.app"
    assert old.exists()
    assert not (home / ".zshrc").exists()
    run()
    second = publish("1.0.1")
    run(FIXTURE_ARCH="x86_64")
    assert subprocess.check_output([str(command), "--version"], text=True).strip() == "shum 1.0.1"
    assert old.exists(), "a loaded old bundle must survive the update"
    assert (home / ".local/share/shum/Shum.app").readlink() == Path(f"versions/{second}/Shum.app")
    before = command.resolve()
    publish("9.0.0", bad=True)
    run(success=False)
    assert command.resolve() == before, "a bad checksum must preserve the working installation"
    unrelated = home / "Library/Application Support/org.Shum.Shum/profiles.json"
    unrelated.parent.mkdir(parents=True)
    unrelated.write_bytes(b"profiles and keys stay intact")
    run("--uninstall")
    assert not command.exists() and not command.is_symlink()
    assert not (home / ".local/share/shum").exists()
    assert unrelated.read_bytes() == b"profiles and keys stay intact"
    assert (home / "calls").read_text().splitlines() == ["refresh 1.0.0", "refresh 1.0.0", "refresh 1.0.1", "uninstall"]
    run("--uninstall")
    (home / ".local/bin/shum").write_text("someone else's binary")
    publish("1.0.2")
    assert "другой установке" in run(success=False)
    assert (home / ".local/bin/shum").read_text() == "someone else's binary"
    print("PASS: unsupported OS; Homebrew refusal; SHA mismatch; signature rejection; PATH output")
    print("PASS: install/repeat/Intel update; stable symlink; immutable old App; refresh invocation")
    print("PASS: failed update preserves installation; idempotent removal preserves data; foreign files protected")
    print("Fixtures only: no Gatekeeper, Bluetooth, real launchd or real release downloads exercised")
