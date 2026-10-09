fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let template = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("native/Info.plist");
        println!("cargo:rerun-if-changed={}", template.display());
        let path =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("Info.plist");
        let plist = std::fs::read_to_string(template).unwrap().replace(
            "__SHUM_VERSION__",
            &std::env::var("CARGO_PKG_VERSION").unwrap(),
        );
        std::fs::write(&path, plist).unwrap();
        println!(
            "cargo:rustc-link-arg-bin=shum=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            path.display()
        );
    }
}
