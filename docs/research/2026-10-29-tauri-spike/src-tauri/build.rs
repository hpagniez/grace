fn main() {
    // Only `api` is an app command, and a capability must grant it (capabilities/default.json).
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["api"])),
    )
    .expect("failed to run tauri-build");
}
