#!/usr/bin/env python3
"""Inspect a bundled installer without installing it or accessing profiles."""
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tarfile
import tempfile
import xml.etree.ElementTree as ET
from package_macos import macho_requirements

package = Path(sys.argv[1]).resolve(strict=True)
manifest = json.loads(package.with_suffix(".json").read_text())
assert hashlib.sha256(package.read_bytes()).hexdigest() == manifest["sha256"]
with tempfile.TemporaryDirectory(prefix="shum-package-check-") as temporary:
    unpacked = Path(temporary) / "expanded"
    subprocess.run(["/usr/sbin/pkgutil", "--expand-full", str(package), str(unpacked)], check=True)
    payload = unpacked / "Shum-cli.pkg/Payload"
    app = payload / "usr/local/libexec/shum/Shum.app"
    binary = app / "Contents/MacOS/shum"
    link = payload / "usr/local/bin/shum"
    assert link.is_symlink() and os.readlink(link) == "/usr/local/libexec/shum/Shum.app/Contents/MacOS/shum"
    files = {str(p.relative_to(payload)) for p in payload.rglob("*") if p.is_file() or p.is_symlink()}
    assert files == {"usr/local/bin/shum", "usr/local/libexec/shum/Shum.app/Contents/Info.plist",
                     "usr/local/libexec/shum/Shum.app/Contents/MacOS/shum",
                     "usr/local/libexec/shum/Shum.app/Contents/Resources/LICENSE",
                     "usr/local/libexec/shum/Shum.app/Contents/_CodeSignature/CodeResources"}, files
    assert os.access(binary, os.X_OK)
    assert (app / "Contents/Resources/LICENSE").read_bytes() == (Path(__file__).resolve().parents[1] / "LICENSE").read_bytes()
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == manifest["binarySha256"]
    minima, images = macho_requirements(binary)
    assert minima == manifest["minimumMacOSByArchitecture"]
    assert sorted(minima) == manifest["architectures"] == ["arm64", "x86_64"]
    assert len(images) == manifest["embeddedExecutables"]
    subprocess.run(["/usr/bin/lipo", "-info", str(binary)], check=True)
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    assert info["CFBundleIdentifier"] == "org.shum.cli"
    assert info["CFBundleVersion"] == info["CFBundleShortVersionString"] == manifest["version"]
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", "--all-architectures", str(app)], check=True)
    signed = subprocess.run(["/usr/bin/codesign", "-dr", "-", str(app)], check=True, text=True, capture_output=True)
    requirement = next(line for line in signed.stdout.splitlines() if line.startswith("designated =>"))
    assert requirement == manifest["designatedRequirement"]
    if not manifest["developmentAdhoc"]:
        assert "certificate leaf" in requirement and "cdhash" not in requirement
        assert manifest["applicationCertificateSha1"].lower() in requirement.lower()
    for architecture in manifest["architectures"]:
        signed = subprocess.run(["/usr/bin/codesign", "--arch", architecture, "-dr", "-", str(app)],
                                check=True, text=True, capture_output=True)
        if not manifest["developmentAdhoc"]:
            assert next(line for line in signed.stdout.splitlines() if line.startswith("designated =>")) == requirement
    receipt = ET.parse(unpacked / "Shum-cli.pkg/PackageInfo").getroot()
    assert receipt.attrib["identifier"] == "org.shum.cli"
    assert receipt.attrib["install-location"] == "/"
    assert receipt.attrib["version"] == manifest["version"]
    distribution = ET.parse(unpacked / "Distribution").getroot()
    assert distribution.find("volume-check/allowed-os-versions/os-version").attrib["min"] == manifest["minimumMacOS"]
    assert distribution.find("options").attrib["hostArchitectures"] == ",".join(manifest["architectures"])
    assert distribution.find("domains").attrib["enable_currentUserHome"] == "false"
    environment = dict(os.environ, PATH="/usr/bin:/bin:/usr/sbin:/sbin")
    result = subprocess.run([str(binary), "--version"], env=environment, check=True, capture_output=True, text=True)
    assert result.stdout.strip() == f'shum {manifest["version"]}'
    result = subprocess.run([str(binary), "--help"], env=environment, check=True, capture_output=True, text=True)
    assert "chats" in result.stdout and "profile" in result.stdout
    archive = package.parent / manifest["archive"]
    archive_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    assert archive.with_suffix(".gz.sha256").read_text().strip() == f"{archive_digest}  {archive.name}"
    with tarfile.open(archive) as bundle:
        assert set(bundle.getnames()) == {"Shum.app", "Shum.app/Contents", "Shum.app/Contents/MacOS",
            "Shum.app/Contents/MacOS/shum", "Shum.app/Contents/Info.plist", "Shum.app/Contents/_CodeSignature",
            "Shum.app/Contents/Resources", "Shum.app/Contents/Resources/LICENSE",
            "Shum.app/Contents/_CodeSignature/CodeResources"}
        assert hashlib.sha256(bundle.extractfile("Shum.app/Contents/MacOS/shum").read()).hexdigest() == manifest["binarySha256"]
    formula = (package.parent / "homebrew-shum/Formula/shum.rb").read_text()
    assert hashlib.sha256(archive.read_bytes()).hexdigest() in formula
    assert "depends_on arch:" not in formula
    assert 'app = buildpath unless app.directory?' in formula
    assert '(prefix/"Shum.app").install app/"Contents"' in formula
    assert 'bin.install_symlink prefix/"Shum.app/Contents/MacOS/shum"' in formula
    assert 'homepage "https://github.com/hiTechTeam/Shum-CLI"' in formula
    assert 'Перед удалением: shum daemon --uninstall' in formula
print("PASS: signed Shum.app, versioned plist, installer symlink, receipt, archive and checksums")
print("PASS: CLI --version and --help with only system executables in PATH")
print("Installer inspected, not installed. Existing profiles not accessed.")
