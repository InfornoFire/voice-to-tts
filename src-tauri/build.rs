use std::env;
use std::path::PathBuf;

use tauri_build::{Attributes, WindowsAttributes};

fn main() {
    let windows = WindowsAttributes::new_without_app_manifest();
    tauri_build::try_build(Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
    embed_manifest();
}

/// Embeds the application manifest in every target so test executables can start.
fn embed_manifest() {
    let is_windows_msvc = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !is_windows_msvc {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let manifest = PathBuf::from(manifest_dir).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}
