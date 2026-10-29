// Grace's thin path on Tauri 2, under the real renderer (../src/main.tsx). The backend is picked at
// start: SPIKE_BACKEND=node runs src/main as a Node sidecar (option 2), anything else the Rust port
// in karate.rs (option 1). SPIKE_PROJECT opens a folder at start, like Grace's last project;
// SPIKE_DATA and SPIKE_VENDOR replace the app data folder and the resource folder.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod karate;
mod sidecar;

use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, RunEvent};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

/// The channel names of src/shared/ipc.ts CH.
const CH_READY: &str = "app:ready";
const CH_OPEN: &str = "project:open";
const CH_RUN: &str = "run:start";
const CH_CANCEL: &str = "run:cancel";
const CH_REPORT: &str = "report:open";
const CH_RUN_REPORT: &str = "history:report";
const CH_EVENT: &str = "app:event";

enum Backend {
    Rust(Arc<karate::Runner>),
    Node(sidecar::Sidecar),
}

fn emitter(app: &AppHandle) -> impl Fn(Value) + Send + Sync + 'static {
    let app = app.clone();
    move |e| {
        let _ = app.emit_to("main", CH_EVENT, e);
    }
}

/// The folder picker runs here, in the shell, as dialog.showOpenDialog does in src/main.ts.
async fn pick_folder(app: &AppHandle) -> Option<PathBuf> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut d = app.dialog().file();
        if let Some(w) = app.get_webview_window("main") {
            d = d.set_parent(&w);
        }
        d.blocking_pick_folder().and_then(|p| p.into_path().ok())
    })
    .await
    .ok()
    .flatten()
}

/// The single entry point of the page, like ipcMain.handle per CH channel. Never rejects.
#[tauri::command]
async fn api(app: AppHandle, channel: String, args: Vec<Value>) -> Value {
    let backend = app.state::<Backend>();
    match &*backend {
        Backend::Node(s) => match channel.as_str() {
            CH_OPEN => match pick_folder(&app).await {
                Some(p) => s.call(CH_OPEN, vec![p.display().to_string().into()]).await,
                None => Value::Null,
            },
            CH_REPORT | CH_RUN_REPORT => {
                if let Some(path) = s.call(&channel, args).await.as_str() {
                    let _ = app.opener().open_path(path, None::<&str>);
                }
                Value::Null
            }
            _ => s.call(&channel, args).await,
        },
        Backend::Rust(r) => {
            let r = r.clone();
            match channel.as_str() {
                CH_READY => r.replay(),
                CH_OPEN => {
                    if let Some(p) = pick_folder(&app).await {
                        let _ = tauri::async_runtime::spawn_blocking(move || r.open(p)).await;
                    }
                }
                CH_RUN => match args.first().and_then(Value::as_str) {
                    Some(key) if key.len() <= 1000 && key.starts_with(['d', 'f', 'o', 's']) => {
                        let key = key.to_string();
                        let _ = tauri::async_runtime::spawn_blocking(move || r.run(&key)).await;
                    }
                    _ => app.emit_to("main", CH_EVENT, serde_json::json!({ "type": "notice", "message": "Invalid request" })).unwrap_or(()),
                },
                CH_CANCEL => r.cancel(),
                _ => app
                    .emit_to("main", CH_EVENT, serde_json::json!({ "type": "notice", "message": format!("{channel}: not in the spike") }))
                    .unwrap_or(()),
            }
            Value::Null
        }
    }
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());
    #[cfg(feature = "plugins")]
    let builder = builder.plugin(tauri_plugin_shell::init());
    builder
        .setup(|app| {
            let handle = app.handle().clone();
            let data = match std::env::var_os("SPIKE_DATA") {
                Some(d) => PathBuf::from(d),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&data)?;
            let vendor = match std::env::var_os("SPIKE_VENDOR") {
                Some(v) => PathBuf::from(v),
                // Tauri canonicalizes current_exe(): `\\?\D:\…`, which Java's -cp cannot read.
                None => {
                    let dir = app.path().resource_dir()?;
                    match dir.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
                        Some(plain) if !plain.starts_with("UNC") => PathBuf::from(plain),
                        _ => dir,
                    }
                }
            };
            let project = std::env::var_os("SPIKE_PROJECT").map(PathBuf::from);
            let backend = if std::env::var("SPIKE_BACKEND").as_deref() == Ok("node") {
                // externalBin puts it next to the app's exe, without the target triple. It inherits
                // SPIKE_PROJECT and opens that folder itself.
                let exe = std::env::current_exe()?.with_file_name("grace-sidecar.exe");
                Backend::Node(sidecar::Sidecar::start(
                    &exe,
                    &data,
                    &vendor,
                    emitter(&handle),
                )?)
            } else {
                let vendor = karate::Vendor {
                    java: vendor.join("jre").join("bin").join("java.exe"),
                    jar: vendor.join("karate.jar"),
                };
                let runner = Arc::new(karate::Runner::new(vendor, data, emitter(&handle)));
                if let Some(p) = project {
                    let r = runner.clone();
                    tauri::async_runtime::spawn_blocking(move || r.open(p));
                }
                Backend::Rust(runner)
            };
            app.manage(backend);
            // Built here, not by Tauri before setup: creating WebView2 takes about 250 ms, and the
            // dry run (or the sidecar) above should not wait for it.
            tauri::WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![api])
        .build(tauri::generate_context!())
        .expect("error while building the Tauri app")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                match &*app.state::<Backend>() {
                    Backend::Node(s) => s.dispose(),
                    Backend::Rust(r) => r.cancel(),
                }
            }
        });
}
