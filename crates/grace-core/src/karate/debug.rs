//! `src/main/karate/debug.ts`: Grace's end of the step debugger's socket (docs/specs/2026-11-02-step-debugger.md, R2),
//! which `listener/grace/DebugListener.java` connects to, and the listener's class name for `command::build_args`.
//!
//! Rust-native (P4 plan, Redesigns, `debug` (session) and `debug` (token)): Node served the connections side by side on
//! its event loop. Here one thread waits in `accept`, and each connection reads its first line on a thread of its own,
//! so a connection that never sends one keeps no other out. The first whose line is the token is kept, the listener
//! stops listening and the other connections are closed; its thread then reads the listener's pause lines, blocked in
//! the OS with no timeout while nothing comes (R2.4). `close` shuts every connection down and wakes `accept` with a
//! connection of its own, then joins it. It does not wait for the connections' threads: on Windows a shutdown does not
//! wake a thread blocked in `recv` (it waited for Windows' 120 s FIN timeout in a test). Each ends on its own, once its
//! peer closes (Java has ended when a run closes its session) or, before the token, after `TOKEN_WAIT`.
use crate::model::events::{Breakpoint, DebugCommand, DebugVar};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};

use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The class of the step debugger's listener (`listener/grace/DebugListener.java`), passed with `--listener`.
pub const DEBUG_LISTENER: &str = "grace.DebugListener";

/// The most a connection may send before its first line ends: the token is 32 characters.
const FIRST_LINE: u64 = 256;
/// How long a connection has to send its first line: Java's listener sends it as it connects. Only before the token:
/// the listener's connection then waits with no limit.
const TOKEN_WAIT: Duration = Duration::from_secs(10);
/// How long a write may wait for the listener to read: its reader thread always reads, so only a stuck Java waits.
const WRITE_LIMIT: Duration = Duration::from_secs(10);

/// The depth-0 test a pause belongs to, as Karate names it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestId {
    pub file: String,
    pub ref_id: String,
}

/// A pause as the listener sends it: the step, the depth-0 test it belongs to, and the variables.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ListenerPause {
    pub file: String,
    pub line: u32,
    pub depth: u32,
    pub prefix: String,
    pub keyword: String,
    pub text: String,
    pub test: TestId,
    pub vars: Vec<DebugVar>,
}

/// None for anything but the listener's pause line.
pub fn parse_pause(json: &str) -> Option<ListenerPause> {
    serde_json::from_str(json).ok()
}

type OnPause = Box<dyn Fn(ListenerPause) + Send + Sync>;
type OnEnd = Box<dyn Fn() + Send + Sync>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Joins a thread, unless it is this one (a callback that drops its session).
fn join(h: JoinHandle<()>) {
    if h.thread().id() != thread::current().id() {
        let _ = h.join();
    }
}

/// Compares every byte, so how long a wrong token took says nothing of where it went wrong.
fn same_token(line: &[u8], token: &str) -> bool {
    line.len() == token.len() && line.iter().zip(token.bytes()).fold(0, |acc, (a, b)| acc | (a ^ b)) == 0
}

/// What the session's threads share, under one lock that no callback is called under.
#[derive(Default)]
struct Conn {
    closed: bool,
    /// False once a connection has sent the token, or the session closed: `accept` then ends.
    listening: bool,
    accept: Option<JoinHandle<()>>,
    /// The listener's connection, to write to.
    socket: Option<TcpStream>,
    /// Every connection still open, by its number, to shut down.
    open: HashMap<u64, TcpStream>,
    next: u64,
    breakpoints: Vec<Breakpoint>,
    /// A pause asked before the listener connected: it goes with the first line.
    pause_first: bool,
}

impl Conn {
    /// Writes a line of JSON to the listener; a write that fails drops it, as the run goes on unpaused (R2.6). True
    /// when it dropped it: the caller then calls `on_end`, out of the lock.
    #[must_use]
    fn write(&mut self, msg: &Value) -> bool {
        if let Some(s) = &mut self.socket
            && s.write_all(format!("{msg}\n").as_bytes()).is_err()
        {
            let _ = s.shutdown(Shutdown::Both);
            self.socket = None;
            return true;
        }
        false
    }
}

struct Shared {
    conn: Mutex<Conn>,
    port: u16,
    token: String,
    on_pause: OnPause,
    on_end: OnEnd,
}

impl Shared {
    /// Wakes `accept` with a connection of its own; false if it could not, and must not be waited for.
    // shortcut: if this loopback connect fails (the system out of ports), the `accept` thread and its port stay until
    // the next connection to it; a non-blocking listener would fix it, if it is ever seen
    fn wake(&self) -> bool {
        TcpStream::connect((Ipv4Addr::LOCALHOST, self.port)).is_ok()
    }

    fn accept(self: &Arc<Self>, listener: TcpListener) {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::ConnectionAborted | io::ErrorKind::ConnectionReset | io::ErrorKind::Interrupted
                    ) =>
                {
                    continue;
                }
                Err(_) => break,
            };
            let mut c = lock(&self.conn);
            if c.closed || !c.listening {
                break; // the stream and the listener close here
            }
            if stream.set_read_timeout(Some(TOKEN_WAIT)).is_err() {
                continue;
            }
            let Ok(clone) = stream.try_clone() else {
                continue;
            };
            let id = c.next;
            c.next += 1;
            c.open.insert(id, clone);
            let shared = Arc::clone(self);
            match thread::Builder::new()
                .name("grace-debug".into())
                .spawn(move || shared.serve(id, stream))
            {
                Ok(_) => {} // not joined: see the module's documentation
                Err(_) => {
                    if let Some(s) = c.open.remove(&id) {
                        let _ = s.shutdown(Shutdown::Both);
                    }
                }
            }
        }
    }

    /// One connection: its first line, then, if it is the listener's, its pause lines until it ends.
    fn serve(self: &Arc<Self>, id: u64, stream: TcpStream) {
        let mut lines = BufReader::new(stream);
        let mut first = Vec::new();
        let read = (&mut lines).take(FIRST_LINE).read_until(b'\n', &mut first);
        let line = first.strip_suffix(b"\n").unwrap_or(&first);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let kept = read.is_ok() && same_token(line, &self.token) && self.keep(id);
        if kept {
            let mut line = String::new();
            loop {
                line.clear();
                if !matches!(lines.read_line(&mut line), Ok(n) if n > 0) {
                    break;
                }
                // a bad line drops the listener: the run goes on unpaused
                let Some(p) = parse_pause(line.trim_end_matches(['\r', '\n'])) else {
                    break;
                };
                // nothing pauses once the listener was dropped (a close, a failed write)
                if lock(&self.conn).socket.is_none() {
                    break;
                }
                (self.on_pause)(p);
            }
        }
        let ended = {
            let mut c = lock(&self.conn);
            if let Some(s) = c.open.remove(&id) {
                let _ = s.shutdown(Shutdown::Both);
            }
            // whoever takes the listener's socket says it ended, once
            kept && c.socket.take().is_some()
        };
        if ended {
            (self.on_end)();
        }
    }

    /// Makes connection `id` the listener's, unless another is or the session closed: stops listening, closes the
    /// others, then sends the breakpoints and a pause asked before.
    fn keep(&self, id: u64) -> bool {
        let accept = {
            let mut c = lock(&self.conn);
            if c.closed || !c.listening {
                return false;
            }
            c.listening = false;
            c.accept.take()
        };
        // not under the lock, which `accept` takes; once it returns, a new connection is refused
        if let Some(h) = accept
            && self.wake()
        {
            join(h);
        }
        let dropped = {
            let mut c = lock(&self.conn);
            if c.closed {
                return false;
            }
            for (_, s) in c.open.extract_if(|other, _| *other != id) {
                let _ = s.shutdown(Shutdown::Both);
            }
            let Some(s) = c.open.get(&id).and_then(|s| s.try_clone().ok()) else {
                return false;
            };
            if s.set_read_timeout(None).is_err() || s.set_write_timeout(Some(WRITE_LIMIT)).is_err() {
                return false;
            }
            c.socket = Some(s);
            let first = json!({ "breakpoints": c.breakpoints, "pause": c.pause_first });
            c.write(&first)
        };
        if dropped {
            (self.on_end)();
        }
        true
    }

    /// `Conn::write` out of the session's lock's callers' way: `on_end` if the write dropped the listener.
    fn write(&self, c: MutexGuard<'_, Conn>, msg: &Value) {
        let mut c = c;
        let dropped = c.write(msg);
        drop(c);
        if dropped {
            (self.on_end)();
        }
    }
}

/// A loopback server for one debug run (R2.2). It serves the first connection whose first line is the token, then
/// stops listening; any other connection is closed. A bad line from the listener drops it: the run goes on unpaused.
/// Dropping it closes it.
pub struct DebugSession {
    shared: Arc<Shared>,
}

impl DebugSession {
    /// Listens on 127.0.0.1 at a port of the system's choice, with a new random token. `on_pause` runs on the
    /// listener's thread for each pause, `on_end` once its connection closes, for whatever reason: nothing pauses
    /// after it. Neither is called under a lock of the session's.
    pub fn open(
        breakpoints: Vec<Breakpoint>,
        on_pause: impl Fn(ListenerPause) + Send + Sync + 'static,
        on_end: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let shared = Arc::new(Shared {
            conn: Mutex::new(Conn {
                listening: true,
                breakpoints,
                ..Conn::default()
            }),
            port: listener.local_addr()?.port(),
            token,
            on_pause: Box::new(on_pause),
            on_end: Box::new(on_end),
        });
        let accept = {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("grace-debug-accept".into())
                .spawn(move || shared.accept(listener))?
        };
        lock(&shared.conn).accept = Some(accept);
        Ok(Self { shared })
    }

    /// True while the listener's connection is served: false before it, and once it was dropped or ended.
    pub fn connected(&self) -> bool {
        lock(&self.shared.conn).socket.is_some()
    }

    pub fn port(&self) -> u16 {
        self.shared.port
    }

    /// What the listener must send first; for Java's command line only, never a log or an event.
    pub fn token(&self) -> &str {
        &self.shared.token
    }

    pub fn set_breakpoints(&self, b: Vec<Breakpoint>) {
        let mut c = lock(&self.shared.conn);
        c.breakpoints = b;
        let msg = json!({ "breakpoints": c.breakpoints });
        self.shared.write(c, &msg);
    }

    /// Before the listener connects, only a pause counts: it goes with the first line.
    pub fn command(&self, cmd: DebugCommand) {
        let mut c = lock(&self.shared.conn);
        if c.socket.is_some() {
            self.shared.write(c, &json!({ "command": cmd }));
        } else if cmd == DebugCommand::Pause {
            c.pause_first = true;
        }
    }

    /// Stops listening and closes every connection, calling `on_end` if the listener was connected; nothing pauses or
    /// is written after it. Again does nothing.
    pub fn close(&self) {
        let (accept, streams, ended) = {
            let mut c = lock(&self.shared.conn);
            c.closed = true;
            c.listening = false;
            let socket = c.socket.take();
            let ended = socket.is_some();
            let streams: Vec<_> = c.open.drain().map(|(_, s)| s).chain(socket).collect();
            (c.accept.take(), streams, ended)
        };
        for s in streams {
            let _ = s.shutdown(Shutdown::Both);
        }
        if let Some(h) = accept
            && self.shared.wake()
        {
            join(h);
        }
        if ended {
            (self.shared.on_end)();
        }
    }
}

impl Drop for DebugSession {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::events::VarKind;
    use serde_json::{Value, json};
    use std::io::{BufRead, BufReader, Write};
    use std::net::Shutdown;
    use std::sync::mpsc::{self, Receiver};
    use std::time::Duration;

    /// How long a test waits for what it expects; it never sleeps a fixed time.
    const LIMIT: Duration = Duration::from_secs(20);

    fn pause_json() -> Value {
        json!({
            "file": "debug.feature",
            "line": 8,
            "depth": 0,
            "prefix": "*",
            "keyword": "def",
            "text": "user = { id: 7 }",
            "test": { "file": "debug.feature", "refId": "[1:6]" },
            "vars": [{ "name": "name", "kind": "string", "value": "grace" }],
        })
    }

    fn bp(file: &str, line: u32) -> Breakpoint {
        Breakpoint {
            file: file.into(),
            line,
        }
    }

    /// A session with a breakpoint on debug.feature:8, and its pauses and ends as they come.
    fn open() -> (DebugSession, Receiver<ListenerPause>, Receiver<()>) {
        let (pause_tx, pauses) = mpsc::channel();
        let (end_tx, ends) = mpsc::channel();
        let s = DebugSession::open(
            vec![bp("debug.feature", 8)],
            move |p| pause_tx.send(p).unwrap(),
            move || {
                let _ = end_tx.send(());
            },
        )
        .unwrap();
        (s, pauses, ends)
    }

    /// A client standing in for the Java listener.
    struct Client {
        socket: TcpStream,
        lines: BufReader<TcpStream>,
    }

    impl Client {
        fn connect(port: u16) -> Self {
            let socket = TcpStream::connect(("127.0.0.1", port)).unwrap();
            socket.set_read_timeout(Some(LIMIT)).unwrap();
            Self {
                lines: BufReader::new(socket.try_clone().unwrap()),
                socket,
            }
        }

        fn send(&mut self, line: &str) {
            self.socket.write_all(format!("{line}\n").as_bytes()).unwrap();
        }

        /// The next line Grace wrote; panics if none comes within the limit.
        fn line(&mut self) -> Value {
            let mut l = String::new();
            assert!(self.lines.read_line(&mut l).unwrap() > 0, "closed");
            serde_json::from_str(&l).unwrap()
        }

        /// True once Grace has closed the connection (an end of stream or a reset), within the limit.
        fn closed(&mut self) -> bool {
            let mut l = String::new();
            match self.lines.read_line(&mut l) {
                Ok(n) => n == 0,
                Err(e) => !matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut),
            }
        }
    }

    mod debug_session {
        use super::*;

        #[test]
        fn sends_the_breakpoints_and_a_pause_asked_before_the_listener_connected() {
            let (s, _p, _e) = open();
            s.command(DebugCommand::Pause);
            s.command(DebugCommand::Over); // ignored: nothing is paused yet
            let mut c = Client::connect(s.port());
            c.send(s.token());
            assert_eq!(
                c.line(),
                json!({ "breakpoints": [{ "file": "debug.feature", "line": 8 }], "pause": true })
            );
        }

        #[test]
        fn closes_a_connection_with_a_wrong_token_and_serves_the_right_one() {
            let (s, _p, _e) = open();
            let mut bad = Client::connect(s.port());
            bad.send("not the token");
            assert!(bad.closed());
            let mut good = Client::connect(s.port());
            good.send(s.token());
            assert_eq!(good.line()["pause"], json!(false));
        }

        #[test]
        fn stops_listening_once_the_listener_is_connected() {
            let (s, _p, _e) = open();
            let mut c = Client::connect(s.port());
            c.send(s.token());
            c.line();
            let late = TcpStream::connect(("127.0.0.1", s.port())).unwrap_err();
            assert_eq!(late.kind(), io::ErrorKind::ConnectionRefused);
        }

        #[test]
        fn passes_a_pause_on_and_writes_commands_and_breakpoints() {
            let (s, pauses, _e) = open();
            let mut c = Client::connect(s.port());
            c.send(s.token());
            c.line();
            c.send(&pause_json().to_string());
            let p = pauses.recv_timeout(LIMIT).unwrap();
            assert_eq!(
                p,
                ListenerPause {
                    file: "debug.feature".into(),
                    line: 8,
                    depth: 0,
                    prefix: "*".into(),
                    keyword: "def".into(),
                    text: "user = { id: 7 }".into(),
                    test: TestId {
                        file: "debug.feature".into(),
                        ref_id: "[1:6]".into()
                    },
                    vars: vec![DebugVar {
                        name: "name".into(),
                        kind: VarKind::String,
                        type_name: None,
                        value: "grace".into(),
                        cut: None
                    }],
                }
            );
            s.command(DebugCommand::Over);
            s.set_breakpoints(Vec::new());
            assert_eq!(
                [c.line(), c.line()],
                [json!({ "command": "over" }), json!({ "breakpoints": [] })]
            );
        }

        #[test]
        fn drops_a_listener_that_sends_a_bad_line() {
            let (s, pauses, ends) = open();
            let mut c = Client::connect(s.port());
            c.send(s.token());
            c.line();
            c.send(&json!({ "file": 1 }).to_string());
            assert!(c.closed());
            ends.recv_timeout(LIMIT).unwrap();
            assert!(pauses.try_recv().is_err());
            s.command(DebugCommand::Continue); // writes to nothing, and does not panic
        }

        #[test]
        fn survives_the_listener_resetting_the_connection_and_says_it_ended() {
            let (s, _p, ends) = open();
            let mut c = Client::connect(s.port());
            c.send(s.token());
            c.line();
            // a line left unread makes the close a reset, as a Java that dies leaves it
            s.set_breakpoints(Vec::new());
            drop(c);
            ends.recv_timeout(LIMIT).unwrap();
            s.set_breakpoints(Vec::new());
        }

        // Added: the listener connects twice (Electron Grace's "accepting one connection", R2.2)
        #[test]
        fn serves_only_the_first_connection_that_sends_the_token() {
            let (s, _p, _e) = open();
            let mut first = Client::connect(s.port());
            let mut second = Client::connect(s.port());
            first.send(s.token());
            first.line();
            second.send(s.token());
            assert!(second.closed());
            s.command(DebugCommand::Continue);
            assert_eq!(first.line(), json!({ "command": "continue" }));
        }

        // Added: a connection that sends nothing holds no other out, nor the close
        #[test]
        fn a_silent_connection_keeps_no_one_out_and_is_closed_with_the_session() {
            let (s, _p, _e) = open();
            let mut silent = Client::connect(s.port());
            let mut c = Client::connect(s.port());
            c.send(s.token());
            c.line();
            s.close();
            assert!(silent.closed());
            assert!(c.closed());
        }

        // Added: a run that the listener never reached ends with no thread left waiting in `accept` (Traps)
        #[test]
        fn closes_with_no_connection_and_stops_listening() {
            let (s, _p, _e) = open();
            let port = s.port();
            s.close();
            s.close();
            let late = TcpStream::connect(("127.0.0.1", port)).unwrap_err();
            assert_eq!(late.kind(), io::ErrorKind::ConnectionRefused);
            s.set_breakpoints(Vec::new());
            s.command(DebugCommand::Pause);
        }

        // Added: the token is never in what the session shows of itself
        #[test]
        fn makes_a_new_token_of_32_hex_digits_for_each_session() {
            let (a, _p, _e) = open();
            let (b, _q, _f) = open();
            assert_eq!(a.token().len(), 32);
            assert!(a.token().bytes().all(|c| c.is_ascii_hexdigit()));
            assert_ne!(a.token(), b.token());
            let mut c = Client::connect(a.port());
            c.send(&a.token()[..31]);
            assert!(c.closed());
            c.socket.shutdown(Shutdown::Both).ok();
        }
    }

    mod parse_pause {
        use super::*;

        #[test]
        fn keeps_a_cut_value_and_the_type_of_another_kind() {
            let mut p = pause_json();
            p["vars"] = json!([
                { "name": "big", "kind": "string", "value": "x", "cut": 36_864 },
                { "name": "d", "kind": "other", "type": "LocalDate", "value": "2026-11-02" },
            ]);
            assert_eq!(
                parse_pause(&p.to_string()).unwrap().vars,
                [
                    DebugVar {
                        name: "big".into(),
                        kind: VarKind::String,
                        type_name: None,
                        value: "x".into(),
                        cut: Some(36_864)
                    },
                    DebugVar {
                        name: "d".into(),
                        kind: VarKind::Other,
                        type_name: Some("LocalDate".into()),
                        value: "2026-11-02".into(),
                        cut: None
                    },
                ]
            );
        }

        #[test]
        fn refuses_a_pause_without_variables_or_with_an_unknown_kind() {
            let mut p = pause_json();
            p.as_object_mut().unwrap().remove("vars");
            assert_eq!(parse_pause(&p.to_string()), None);
            p["vars"] = json!([{ "name": "a", "kind": "date", "value": "x" }]);
            assert_eq!(parse_pause(&p.to_string()), None);
            assert_eq!(parse_pause("not json"), None);
        }
    }
}
