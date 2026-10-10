//! Launch the daemon as a macOS application, so Bluetooth permission belongs
//! to Shum instead of a terminal which may lack a Bluetooth purpose string.
use crate::i18n::t;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    process::Command,
};

const PLIST: &str = include_str!(concat!(env!("OUT_DIR"), "/Info.plist"));

pub(crate) fn prepare(root: &Path) -> Result<PathBuf> {
    let executable = std::env::current_exe()?;
    if let Some(app) = installed_bundle(&executable)? {
        return Ok(app);
    }
    let mut source = File::open(&executable)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    digest.update(PLIST);
    // Keep running versions intact, including their signed bundle metadata.
    // Replacing a live bundle can invalidate macOS's privacy attribution.
    let cache = root.join("services");
    fs::create_dir_all(&cache)?;
    let version = cache.join(hex::encode(digest.finalize()));
    let app = version.join("Shum.app");
    if app.is_dir() {
        return Ok(app);
    }
    let staging = tempfile::tempdir_in(&cache)?;
    let bundle = staging.path().join("Shum.app");
    let contents = bundle.join("Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    source.rewind()?;
    let binary = contents.join("MacOS/shum");
    let mut destination = File::create(&binary)?;
    std::io::copy(&mut source, &mut destination)?;
    fs::set_permissions(&binary, source.metadata()?.permissions())?;
    drop(destination);
    File::create(contents.join("Info.plist"))?.write_all(PLIST.as_bytes())?;
    let signed = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--identifier", "org.shum.cli"])
        .arg(&bundle)
        .output()
        .context(t("Не удалось подписать локальную службу Shum"))?;
    if !signed.status.success() {
        bail!(
            "{}",
            crate::i18n::format(
                "Подпись службы Shum: {}",
                &[(
                    "0",
                    crate::terminal::safe(&String::from_utf8_lossy(&signed.stderr)).to_string()
                )]
            )
        );
    }
    match fs::rename(staging.path(), &version) {
        Ok(()) => {}
        // Concurrent CLI launches can prepare the same immutable bundle.
        Err(_) if app.is_dir() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(app)
}

pub(crate) fn launch(root: &Path, id: &str) -> Result<()> {
    let root = root.canonicalize()?;
    let bundle = prepare(&root)?;
    let log = root.join(id).join("daemon.log");
    let result = Command::new("/usr/bin/open")
        .args(["-n", "-g", "-a"])
        .arg(bundle)
        .arg("--stdout")
        .arg(&log)
        .arg("--stderr")
        .arg(&log)
        .args(["--args", "--data-dir"])
        .arg(&root)
        .args(["--profile", id, "daemon", "--run"])
        .output()
        .context(t("Не удалось открыть службу Shum через macOS"))?;
    if !result.status.success() {
        bail!(
            "{}",
            crate::i18n::format(
                "Запуск службы Shum: {}",
                &[(
                    "0",
                    crate::terminal::safe(&String::from_utf8_lossy(&result.stderr)).to_string()
                )]
            )
        );
    }
    Ok(())
}

// macOS may report bin/shum rather than the symlink's target. Resolve it before
// inspecting the enclosing signed bundle; otherwise an installation looks like Cargo.
fn installed_bundle(executable: &Path) -> Result<Option<PathBuf>> {
    let executable = executable.canonicalize()?;
    let Some(macos) = executable.parent() else {
        return Ok(None);
    };
    let Some(contents) = macos.parent() else {
        return Ok(None);
    };
    let Some(app) = contents.parent() else {
        return Ok(None);
    };
    if macos.file_name() != Some("MacOS".as_ref())
        || contents.file_name() != Some("Contents".as_ref())
        || app.file_name() != Some("Shum.app".as_ref())
    {
        return Ok(None);
    }
    verify(app)?;
    let stable = stable_bundle(app);
    if stable != app && stable.canonicalize().ok().as_deref() != Some(app) {
        bail!("{}", t("Стабильная ссылка установки не указывает на запущенный Shum.app. Запустите актуальную команду shum."));
    }
    Ok(Some(stable))
}

fn stable_bundle(app: &Path) -> PathBuf {
    // <brew>/Cellar/shum/<version>/Shum.app -> <brew>/opt/shum/Shum.app
    let Some(version) = app.parent() else {
        return app.into();
    };
    let Some(formula) = version.parent() else {
        return app.into();
    };
    let Some(cellar) = formula.parent() else {
        return app.into();
    };
    if formula.file_name() == Some("shum".as_ref()) && cellar.file_name() == Some("Cellar".as_ref())
    {
        if let Some(prefix) = cellar.parent() {
            return prefix.join("opt/shum/Shum.app");
        }
    }
    // ~/.local/share/shum/versions/<archive-sha256>/Shum.app -> stable App link.
    if formula.file_name() == Some("versions".as_ref())
        && cellar.file_name() == Some("shum".as_ref())
        && version
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.len() == 64 && name.bytes().all(|c| c.is_ascii_hexdigit()))
        && cellar.join(".shum-installer").is_file()
    {
        return cellar.join("Shum.app");
    }
    app.into()
}

fn verify(app: &Path) -> Result<()> {
    let verified = Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict", "-R", "=identifier \"org.shum.cli\""])
        .arg(app)
        .output()?;
    if !verified.status.success() {
        bail!(
            "{}",
            crate::i18n::format(
                "Подпись установленного Shum.app недействительна: {}",
                &[(
                    "0",
                    crate::terminal::safe(&String::from_utf8_lossy(&verified.stderr)).to_string()
                )]
            )
        );
    }
    Ok(())
}

pub(crate) fn service_build(root: &Path) -> Result<crate::identity::Build> {
    let app = prepare(root)?;
    Ok(crate::identity::Build {
        version: env!("CARGO_PKG_VERSION").into(),
        sha256: crate::identity::hash(&app.join("Contents/MacOS/shum"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_bundle_is_reused_and_wrong_identifier_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Shum.app");
        let binary = app.join("Contents/MacOS/shum");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::copy("/usr/bin/true", &binary).unwrap();
        fs::write(app.join("Contents/Info.plist"), PLIST).unwrap();
        let sign = |identifier: &str| {
            let output = Command::new("/usr/bin/codesign")
                .args(["--force", "--sign", "-", "--identifier", identifier])
                .arg(&app)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        sign("org.shum.cli");
        assert_eq!(
            installed_bundle(&binary).unwrap(),
            Some(app.canonicalize().unwrap())
        );
        let bin = dir.path().join("bin/shum");
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&binary, &bin).unwrap();
        assert_eq!(
            installed_bundle(&bin).unwrap(),
            Some(app.canonicalize().unwrap())
        );
        sign("org.example.other");
        assert!(installed_bundle(&binary).is_err());
    }

    #[test]
    fn user_installation_uses_stable_app_link() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".local/share/shum");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".shum-installer"), "shum-install-v1").unwrap();
        let app = root.join("versions").join("a".repeat(64)).join("Shum.app");
        assert_eq!(stable_bundle(&app), root.join("Shum.app"));
        let other = root.join("versions/not-a-hash/Shum.app");
        assert_eq!(stable_bundle(&other), other);
    }

    #[test]
    fn homebrew_path_survives_cellar_cleanup() {
        assert_eq!(
            stable_bundle(Path::new("/opt/homebrew/Cellar/shum/0.1.5/Shum.app")),
            Path::new("/opt/homebrew/opt/shum/Shum.app")
        );
        assert_eq!(
            stable_bundle(Path::new("/usr/local/Cellar/shum/1.2.3/Shum.app")),
            Path::new("/usr/local/opt/shum/Shum.app")
        );
        let cargo = Path::new("/tmp/data/services/abc/Shum.app");
        assert_eq!(stable_bundle(cargo), cargo);
    }
}
