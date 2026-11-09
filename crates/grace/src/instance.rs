//! One Grace per data folder (`main.ts`'s `requestSingleInstanceLock`): a second one would kill the first one's run as
//! an orphan and share its run folders. It hands over to the first window instead, which comes to the front.
//!
//! Rust-native (P2 plan, Redesigns): the first Grace holds an exclusive lock on `<data folder>/instance.lock`
//! (`File::try_lock`, which the system drops when the process ends, even in a crash) and listens on a loopback port
//! that it writes to `instance.port`. A second Grace fails to take the lock, connects to that port, says `show` and
//! quits. The listening thread waits in `accept`: it never wakes on its own (ADR 0030).
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What a second start says to the first.
const SHOW: &str = "show";

/// The first Grace of a data folder: its lock, held while it lives, and its listener.
pub struct Primary {
    _lock: File,
    listener: TcpListener,
}

pub enum Start {
    /// This is the only Grace of the folder.
    Primary(Primary),
    /// Another Grace has the folder; it was asked to come to the front (when it could be reached).
    Secondary,
}

fn lock_path(data_dir: &Path) -> PathBuf {
    data_dir.join("instance.lock")
}

fn port_path(data_dir: &Path) -> PathBuf {
    data_dir.join("instance.port")
}

/// Takes the data folder, or hands over to the Grace that has it.
pub fn start(data_dir: &Path) -> std::io::Result<Start> {
    fs::create_dir_all(data_dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(lock_path(data_dir))?;
    if lock.try_lock().is_err() {
        tell_primary(data_dir);
        return Ok(Start::Secondary);
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    fs::write(port_path(data_dir), listener.local_addr()?.port().to_string())?;
    Ok(Start::Primary(Primary { _lock: lock, listener }))
}

/// Asks the first Grace to come to the front. It may have taken the lock a moment before writing its port: try again
/// for up to two seconds.
fn tell_primary(data_dir: &Path) {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let port = fs::read_to_string(port_path(data_dir))
            .ok()
            .and_then(|p| p.trim().parse::<u16>().ok());
        if let Some(port) = port
            && let Ok(mut s) = TcpStream::connect_timeout(&(Ipv4Addr::LOCALHOST, port).into(), Duration::from_secs(1))
        {
            let _ = writeln!(s, "{SHOW}");
            return;
        }
        if Instant::now() > until {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl Primary {
    /// Calls `show` each time another Grace starts on the folder, from a thread that waits in `accept`.
    pub fn listen(self, show: impl Fn() + Send + 'static) {
        std::thread::Builder::new()
            .name("grace-instance".into())
            .spawn(move || {
                let _lock = self._lock;
                for stream in self.listener.incoming().flatten() {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut line = String::new();
                    if BufReader::new(stream).read_line(&mut line).is_ok() && line.trim() == SHOW {
                        show();
                    }
                }
            })
            .expect("the instance thread starts");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("grace-instance-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_first_start_has_the_folder_and_a_second_hands_over_to_it() {
        let dir = folder("two");
        let Start::Primary(first) = start(&dir).unwrap() else {
            panic!("the first start has the folder")
        };
        let (tx, rx) = mpsc::channel();
        first.listen(move || tx.send(()).unwrap());
        assert!(matches!(start(&dir).unwrap(), Start::Secondary));
        rx.recv_timeout(Duration::from_secs(5))
            .expect("the first Grace was told");
    }

    #[test]
    fn another_data_folder_is_another_grace() {
        let (a, b) = (folder("a"), folder("b"));
        let first = start(&a).unwrap();
        assert!(matches!(first, Start::Primary(_)));
        assert!(matches!(start(&b).unwrap(), Start::Primary(_)));
    }

    #[test]
    fn the_folder_is_free_again_once_the_first_grace_is_gone() {
        let dir = folder("gone");
        let first = start(&dir).unwrap();
        drop(first);
        assert!(matches!(start(&dir).unwrap(), Start::Primary(_)));
    }
}
