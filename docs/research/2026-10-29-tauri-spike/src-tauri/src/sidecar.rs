//! Option 2: src/main unchanged in a Node sidecar (../sidecar/main.ts). One JSON line per request
//! on its stdin, one per answer or app event on its stdout. Closing stdin makes it dispose and quit.
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

pub struct Sidecar {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    pending: Pending,
    next: AtomicU64,
}

impl Sidecar {
    pub fn start(
        exe: &Path,
        data: &Path,
        vendor: &Path,
        emit: impl Fn(Value) + Send + 'static,
    ) -> std::io::Result<Self> {
        let mut child = Command::new(exe)
            .env("GRACE_DATA", data)
            .env("GRACE_VENDOR", vendor)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: node.exe is a console program
            .spawn()?;
        let stdout = child.stdout.take().unwrap();
        let pending: Pending = Arc::default();
        let answers = pending.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(e) = msg.get("event") {
                    emit(e.clone());
                } else if let Some(tx) = msg["id"]
                    .as_u64()
                    .and_then(|id| answers.lock().unwrap().remove(&id))
                {
                    let _ = tx.send(msg["result"].clone());
                }
            }
        });
        let stdin = child.stdin.take();
        Ok(Sidecar {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            pending,
            next: AtomicU64::new(1),
        })
    }

    pub async fn call(&self, channel: &str, args: Vec<Value>) -> Value {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let line = json!({ "id": id, "channel": channel, "args": args }).to_string() + "\n";
        let sent = match self.stdin.lock().unwrap().as_mut() {
            Some(w) => w.write_all(line.as_bytes()).and_then(|_| w.flush()).is_ok(),
            None => false,
        };
        if !sent {
            self.pending.lock().unwrap().remove(&id);
            return Value::Null;
        }
        rx.await.unwrap_or(Value::Null)
    }

    /// Like Electron's before-quit: the sidecar cancels a run and quits; given up after 10 s.
    pub fn dispose(&self) {
        drop(self.stdin.lock().unwrap().take());
        let mut child = self.child.lock().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if let Ok(Some(_)) = child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        crate::karate::taskkill(child.id());
    }
}
