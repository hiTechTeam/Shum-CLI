//! Cache the running executable's identity before its installation can be replaced.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, sync::OnceLock};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    pub version: String,
    pub sha256: String,
}
static CURRENT: OnceLock<Build> = OnceLock::new();

pub fn current() -> Result<&'static Build> {
    if let Some(build) = CURRENT.get() {
        return Ok(build);
    }
    let sha256 = hash(&std::env::current_exe()?)?;
    let _ = CURRENT.set(Build {
        version: env!("SHUM_VERSION").into(),
        sha256,
    });
    Ok(CURRENT.get().expect("build identity initialized"))
}

pub fn hash(path: &std::path::Path) -> Result<String> {
    let mut binary = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let size = binary.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    Ok(hex::encode(digest.finalize()))
}

pub fn matches(snapshot: &serde_json::Value, build: &Build) -> bool {
    serde_json::from_value::<Build>(snapshot["build"].clone())
        .is_ok_and(|running| running == *build)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_version_different_hash_requires_replacement() {
        let build = Build {
            version: "0.1.5".into(),
            sha256: "a".repeat(64),
        };
        assert!(matches(&serde_json::json!({"build": build}), &build));
        assert!(!matches(
            &serde_json::json!({"build": {"version": "0.1.5", "sha256": "b".repeat(64)}}),
            &build
        ));
        assert!(!matches(
            &serde_json::json!({"build": {"version": "0.1.4", "sha256": build.sha256}}),
            &build
        ));
        assert!(!matches(&serde_json::json!({}), &build));
    }
}
