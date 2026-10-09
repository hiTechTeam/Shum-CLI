#!/usr/bin/env python3
"""Build a universal macOS installer. End users do not need developer tools."""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
from urllib.parse import urlsplit
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def thin_requirements(data):
    """Inspect the CLI and embedded BLE executable, including deployment targets."""
    images = []
    offset = 0
    while True:
        offset = data.find(b"\xcf\xfa\xed\xfe", offset)
        if offset < 0:
            break
        start = offset
        offset += 4
        if start + 32 > len(data):
            continue
        _, cpu, _, kind, count, size, _, _ = struct.unpack_from("<8I", data, start)
        if kind != 2 or cpu not in (0x100000C, 0x1000007) or not 0 < count < 4096:
            continue
        pos = start + 32
        end = pos + size
        if end > len(data):
            continue
        minimum = None
        dependencies = []
        for _ in range(count):
            if pos + 8 > end:
                break
            cmd, length = struct.unpack_from("<2I", data, pos)
            if length < 8 or pos + length > end:
                break
            if cmd == 0x32 and length >= 24:
                system, version = struct.unpack_from("<2I", data, pos + 8)
                if system == 1:
                    minimum = (version >> 16, (version >> 8) & 255, version & 255)
            elif cmd == 0x24 and length >= 16:
                version = struct.unpack_from("<I", data, pos + 8)[0]
                minimum = (version >> 16, (version >> 8) & 255, version & 255)
            elif cmd in (0xC, 0x80000018, 0x8000001F) and length >= 24:
                name_offset = struct.unpack_from("<I", data, pos + 8)[0]
                if 24 <= name_offset < length:
                    raw = data[pos + name_offset:pos + length].split(b"\0", 1)[0]
                    dependencies.append(raw.decode("utf-8"))
            pos += length
        else:
            if pos == end and minimum:
                images.append({"offset": start, "cpu": cpu, "minimum": minimum,
                               "dependencies": dependencies})
    if len(images) < 2 or images[0]["offset"] != 0:
        raise ValueError("Ожидался macOS CLI со встроенным BLE-бинарником")
    if len({image["cpu"] for image in images}) != 1:
        raise ValueError("Архитектуры CLI и встроенной службы не совпадают")
    for image in images:
        for dependency in image["dependencies"]:
            if not dependency.startswith(("/System/Library/", "/usr/lib/")):
                raise ValueError(f"Внешняя зависимость мешает автономной установке: {dependency}")
    return images


def macho_requirements(path):
    """Validate both fat slices and the architecture of each embedded BLE helper."""
    data = path.read_bytes()
    if data[:4] != b"\xca\xfe\xba\xbe":
        raise ValueError("Нужен универсальный Mach-O: arm64 и x86_64")
    count, = struct.unpack_from(">I", data, 4)
    if count != 2:
        raise ValueError("Нужны ровно две архитектуры: arm64 и x86_64")
    minima = {}
    images = []
    ranges = []
    for i in range(count):
        cpu, _, start, size, _ = struct.unpack_from(">5I", data, 8 + 20 * i)
        if cpu not in (0x100000C, 0x1000007) or start < 8 + 20 * count or start + size > len(data):
            raise ValueError("Недействительный универсальный Mach-O")
        if any(start < end and begin < start + size for begin, end in ranges):
            raise ValueError("Пересекающиеся архитектуры Mach-O")
        ranges.append((start, start + size))
        name = "arm64" if cpu == 0x100000C else "x86_64"
        if name in minima:
            raise ValueError("Повторная архитектура Mach-O")
        embedded = thin_requirements(data[start:start + size])
        if embedded[0]["cpu"] != cpu:
            raise ValueError("Архитектура среза не соответствует заголовку")
        minimum = max(image["minimum"] for image in embedded)
        if minimum != (15, 0, 0):
            raise ValueError(f"{name}: требуется deployment target macOS 15.0, получено {minimum}")
        minima[name] = ".".join(map(str, minimum))
        images.extend(dict(image, architecture=name, offset=start + image["offset"]) for image in embedded)
    return minima, images


def build_universal():
    """Core's pinned BLE build script needs an explicit Swift cross target."""
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target/macos-package")).resolve()
    installed = run("rustup", "target", "list", "--installed", capture_output=True).stdout.splitlines()
    targets = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64"}
    missing = targets.keys() - set(installed)
    if missing:
        raise ValueError("Цель Rust отсутствует; сначала согласуйте загрузку: " + ", ".join(sorted(missing)))
    sdk = run("/usr/bin/xcrun", "--sdk", "macosx", "--show-sdk-path", capture_output=True).stdout.strip()
    with tempfile.TemporaryDirectory(prefix="shum-cross-tools-") as temporary:
        wrapper = Path(temporary) / "xcrun"
        wrapper.write_text('#!/bin/sh\ncase "${1-}" in\n'
                           'swiftc) shift; exec /usr/bin/xcrun swiftc -target "$SHUM_SWIFT_TARGET" "$@" ;;\n'
                           '*) exec /usr/bin/xcrun "$@" ;;\nesac\n')
        wrapper.chmod(0o755)
        binaries = []
        for rust_target, architecture in targets.items():
            env = dict(os.environ, PATH=temporary + os.pathsep + os.environ["PATH"],
                       MACOSX_DEPLOYMENT_TARGET="15.0", SDKROOT=sdk,
                       SHUM_SWIFT_TARGET=f"{architecture}-apple-macosx15.0")
            # cc-rs uses Cargo's TARGET and these flags for secp256k1, sqlite and zlib.
            key = rust_target.replace("-", "_")
            env[f"CFLAGS_{key}"] = "-mmacosx-version-min=15.0"
            env[f"CXXFLAGS_{key}"] = "-mmacosx-version-min=15.0"
            run("cargo", "build", "--locked", "--release", "-p", "shum-cli",
                "--target", rust_target, "--target-dir", str(target_dir), cwd=ROOT, env=env)
            binaries.append(target_dir / rust_target / "release/shum")
        binary = target_dir / "shum-universal"
        run("/usr/bin/lipo", "-create", *(str(p) for p in binaries), "-output", str(binary))
    return binary


def application_identity(name):
    # Read only the named public certificate; never export its private key.
    result = run("/usr/bin/security", "find-certificate", "-a", "-c", name, "-p", capture_output=True)
    certificates = re.findall(r"-----BEGIN CERTIFICATE-----\s*(.*?)\s*-----END CERTIFICATE-----", result.stdout, re.S)
    fingerprints = {hashlib.sha1(base64.b64decode(cert)).hexdigest().upper() for cert in certificates}
    if len(fingerprints) != 1:
        raise ValueError(f"Нужен один публичный сертификат {name!r}, найдено {len(fingerprints)}")
    return fingerprints.pop()


def sign_application(app, identity, development=False):
    certificate = None if development else application_identity(identity)
    requirement = ('designated => identifier "org.shum.cli" and certificate leaf = H"'
                   + certificate + '"') if certificate else None
    command = ["/usr/bin/codesign", "--force", "--sign", certificate or "-",
               "--identifier", "org.shum.cli", "--timestamp=none"]
    if requirement:
        command += ["--requirements", "=" + requirement]
    run(*command, str(app))
    run("/usr/bin/codesign", "--verify", "--strict", "--all-architectures", str(app))
    result = run("/usr/bin/codesign", "-dr", "-", str(app), capture_output=True)
    designated = next((line for line in result.stdout.splitlines() if line.startswith("designated =>")), "")
    if not designated:
        raise ValueError("codesign не вернул designated requirement")
    if certificate:
        if 'identifier "org.shum.cli"' not in designated or certificate.lower() not in designated.lower() or "cdhash" in designated:
            raise ValueError(f"Нестабильное требование подписи: {designated}")
    for architecture in ("arm64", "x86_64"):
        result = run("/usr/bin/codesign", "--arch", architecture, "-dr", "-", str(app), capture_output=True)
        other = next((line for line in result.stdout.splitlines() if line.startswith("designated =>")), "")
        if certificate and other != designated:
            raise ValueError(f"Требования подписи различаются для {architecture}")
    return certificate, designated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="Упаковать уже проверенный бинарник")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--sign-application", default="Shum CLI Signing", help="Самоподписанный сертификат подписи кода")
    parser.add_argument("--development-adhoc", action="store_true", help="Только локальная проверка, непригодна для выпуска")
    parser.add_argument("--sign-installer", help="Имя сертификата Developer ID Installer")
    parser.add_argument("--release-base-url", help="HTTPS-каталог опубликованных артефактов")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("Установщик macOS собирается на macOS")
    if args.development_adhoc and args.release_base_url:
        parser.error("Ad hoc подпись разрешена только для локальной проверки")
    if args.release_base_url:
        parsed = urlsplit(args.release_base_url)
        if parsed.scheme != "https" or not parsed.netloc or parsed.query or parsed.fragment:
            parser.error("Для релиза нужен HTTPS URL каталога без query и fragment")
    version = re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"',
                        (ROOT / "Cargo.toml").read_text(), re.M).group(1)
    source_revision = None
    if args.binary:
        binary = args.binary.expanduser().resolve(strict=True)
    else:
        binary = build_universal()
        source_revision = run("git", "describe", "--always", "--dirty", cwd=ROOT,
                              capture_output=True).stdout.strip()
    minima, images = macho_requirements(binary)
    architecture = "universal"
    minimum = "15.0.0"
    macos_name = "sequoia"
    run("/usr/bin/lipo", "-info", str(binary))
    clean_env = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": os.environ["HOME"]}
    actual_version = run(str(binary), "--version", env=clean_env, capture_output=True).stdout.strip()
    if actual_version != f"shum {version}":
        parser.error(f"Версия бинарника {actual_version!r} не совпадает с {version}")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    suffix = "" if args.sign_installer else "-unsigned"
    package = output / f"Shum-CLI-{version}-macOS-{architecture}{suffix}.pkg"
    archive = output / "shum-macos-universal.tar.gz"
    if package.exists() or archive.exists():
        parser.error(f"Файл уже существует: {package}")
    with tempfile.TemporaryDirectory(prefix="shum-package-") as temporary:
        stage = Path(temporary)
        app = stage / "root/usr/local/libexec/shum/Shum.app"
        executable = app / "Contents/MacOS/shum"
        executable.parent.mkdir(parents=True)
        shutil.copy2(binary, executable)
        executable.chmod(0o755)
        template = (ROOT / "native/Info.plist").read_text().replace("__SHUM_VERSION__", version)
        (app / "Contents/Info.plist").write_text(template)
        resources = app / "Contents/Resources"
        resources.mkdir()
        shutil.copy2(ROOT / "LICENSE", resources / "LICENSE")
        certificate, designated = sign_application(app, args.sign_application, args.development_adhoc)
        binary_digest = hashlib.sha256(executable.read_bytes()).hexdigest()
        link = stage / "root/usr/local/bin/shum"
        link.parent.mkdir(parents=True)
        link.symlink_to("/usr/local/libexec/shum/Shum.app/Contents/MacOS/shum")
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(app, arcname="Shum.app")
        component = stage / "Shum-cli.pkg"
        components = stage / "components.plist"
        components.write_bytes(plistlib.dumps([{
            "RootRelativeBundlePath": "usr/local/libexec/shum/Shum.app",
            "BundleIsRelocatable": False,
            "BundleIsVersionChecked": True,
            "BundleHasStrictIdentifier": True,
            "BundleOverwriteAction": "upgrade",
        }]))
        run("/usr/bin/pkgbuild", "--root", str(stage / "root"), "--component-plist", str(components), "--identifier", "org.shum.cli",
            "--version", version, "--install-location", "/", "--ownership", "recommended",
            str(component))
        requirements = stage / "requirements.plist"
        requirements.write_bytes(plistlib.dumps({"arch": sorted(minima), "os": [minimum]}))
        distribution = stage / "Distribution.xml"
        run("/usr/bin/productbuild", "--synthesize", "--product", str(requirements),
            "--package", str(component), str(distribution))
        tree = ET.parse(distribution)
        document = tree.getroot()
        ET.SubElement(document, "title").text = "Shum CLI"
        ET.SubElement(document, "welcome", {"file": "welcome.html", "mime-type": "text/html"})
        ET.SubElement(document, "conclusion", {"file": "conclusion.html", "mime-type": "text/html"})
        ET.SubElement(document, "domains", {"enable_anywhere": "false",
                       "enable_currentUserHome": "false", "enable_localSystem": "true"})
        tree.write(distribution, encoding="utf-8", xml_declaration=True)
        resources = stage / "resources"
        resources.mkdir()
        html = '<html lang="ru"><meta charset="utf-8"><body style="font:15px -apple-system;line-height:1.5">'
        (resources / "welcome.html").write_text(html + '<h1>Shum в терминале</h1>'
            '<p>Установщик добавит команду <b>shum</b> в /usr/local/bin.</p>'
            '<p>Профили и переписка сохраняются.</p>'
            f'<p>Архитектура: {architecture}. Минимальная macOS: {minimum}.</p></body></html>')
        (resources / "conclusion.html").write_text(html + '<h1>Shum установлен</h1>'
            '<p>Откройте Terminal или Warp и выполните:</p><pre>shum</pre>'
            '<p>Первый запуск предложит создать профиль. Для списка чатов:</p><pre>shum chats</pre>'
            '<p>Если раньше вы ставили Shum через Cargo, команда <code>command -v shum</code> '
            'покажет используемую копию. Новая версия находится в <code>/usr/local/bin/shum</code>.</p>'
            '</body></html>')
        command = ["/usr/bin/productbuild", "--distribution", str(distribution),
                   "--package-path", str(stage), "--resources", str(resources)]
        if args.sign_installer:
            command += ["--sign", args.sign_installer]
        run(*command, str(package))
    digest = hashlib.sha256(package.read_bytes()).hexdigest()
    package.with_suffix(".pkg.sha256").write_text(f"{digest}  {package.name}\n")
    manifest = {"version": version, "architecture": architecture, "architectures": sorted(minima),
                "minimumMacOSByArchitecture": minima, "minimumMacOS": minimum, "archive": archive.name,
                "package": package.name, "sha256": digest,
                "binarySha256": binary_digest,
                "applicationCertificateSha1": certificate, "designatedRequirement": designated,
                "developmentAdhoc": args.development_adhoc,
                "sourceRevision": source_revision, "signedInstaller": bool(args.sign_installer),
                "notarized": False, "embeddedExecutables": len(images)}
    package.with_suffix(".json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    archive_digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(".gz.sha256").write_text(f"{archive_digest}  {archive.name}\n")
    formula_url = (args.release_base_url.rstrip("/") + "/" + archive.name
                   if args.release_base_url else archive.as_uri())
    # Quote Ruby literals without enabling interpolation in user-supplied URLs.
    ruby_url = "'" + formula_url.replace("\\", "\\\\").replace("'", "\\'") + "'"
    tap = output / "homebrew-shum"
    formula = tap / "Formula/shum.rb"
    formula.parent.mkdir(parents=True, exist_ok=True)
    formula.write_text(f'''class Shum < Formula
  desc "Private messenger in your terminal"
  homepage "https://github.com/hiTechTeam/Shum-CLI"
  url {ruby_url}
  version "{version}"
  sha256 "{archive_digest}"
  license "MIT"

  depends_on macos: :{macos_name}

  def install
    app = buildpath/"Shum.app"
    app = buildpath unless app.directory?
    (prefix/"Shum.app").install app/"Contents"
    bin.install_symlink prefix/"Shum.app/Contents/MacOS/shum"
  end

  def caveats
    "Перед удалением: shum daemon --uninstall"
  end

  test do
    assert_equal "shum {version}", shell_output("#{{bin}}/shum --version").strip
  end
end
''')
    (tap / "README.md").write_text(
        "# Shum Homebrew tap\n\n"
        "Формула устанавливает подписанный Shum.app и проверяет SHA-256.\n\n"
        "После публикации tap hiTechTeam/homebrew-shum:\n\n"
        "```sh\nbrew install hitechteam/shum/shum\nbrew upgrade shum\n```\n\n"
        "При следующем запуске CLI служба обновится автоматически.\n\n"
        "Удаление:\n\n```sh\nshum daemon --uninstall\nbrew uninstall shum\n```\n\n"
        "Данные сохраняются в `~/Library/Application Support/org.Shum.Shum`.\n"
        "Для полного удаления после остановки служб удалите этот каталог вручную.\n"
        "При профилях в Keychain также удалите только их ключи Shum в Связке ключей.\n"
        "Если формула уже удалена, для каждого файла `~/Library/LaunchAgents/org.shum.cli.*.plist` "
        "выполните `launchctl bootout gui/$(id -u) /полный/путь/к/файлу.plist`, затем удалите файл. "
        "`KeepAlive=false`: бесконечного перезапуска нет.\n"
        + ("URL указывает на локальный архив. Перед публикацией пересоберите с "
           "`--release-base-url` и адресом каталога артефактов.\n" if not args.release_base_url else ""))
    print(json.dumps(manifest, ensure_ascii=False, indent=2))
    print(package)
    print(archive)
    print(formula)


if __name__ == "__main__":
    main()
