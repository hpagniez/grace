//! The versions the console's banner names (`Banner.tsx`'s `__KK_VERSIONS__`): Grace's from `package.json`, Karate's
//! and Java's from the pins in `scripts/vendor.json`, so the banner names what the release bundles. On Windows, Grace's
//! icon in `Grace.exe` (`build/res.rs`).
use std::path::Path;

#[path = "build/res.rs"]
mod res;

fn json(path: &Path) -> serde_json::Value {
    println!("cargo:rerun-if-changed={}", path.display());
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let app = json(&root.join("package.json"));
    let vendor = json(&root.join("scripts/vendor.json"));
    let text = |v: &serde_json::Value| v.as_str().expect("a version string").to_string();
    println!("cargo:rustc-env=GRACE_VERSION={}", text(&app["version"]));
    println!(
        "cargo:rustc-env=GRACE_KARATE_VERSION={}",
        text(&vendor["karate"]["version"])
    );
    println!("cargo:rustc-env=GRACE_JAVA_VERSION={}", text(&vendor["jre"]["version"]));
    // MSVC's linker takes a .res file among its inputs, as it takes an object file
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let ico = root.join("assets/icon.ico");
        println!("cargo:rerun-if-changed={}", ico.display());
        let bytes = std::fs::read(&ico).unwrap_or_else(|e| panic!("{}: {e}", ico.display()));
        let icon = res::icon(&bytes).unwrap_or_else(|e| panic!("{}: {e}", ico.display()));
        let out = Path::new(&std::env::var("OUT_DIR").expect("Cargo sets OUT_DIR")).join("icon.res");
        std::fs::write(&out, icon).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
        println!("cargo:rustc-link-arg-bins={}", out.display());
    }
}
