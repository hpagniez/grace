//! `src/main/karate/events.ts`: Karate's event stream, `<output>/karate-json/karate-events.jsonl`. `JsonlTail` reads
//! the lines that have grown since its last read; `Tailer` follows the file on its own thread for as long as a run
//! goes (docs/inventory/karate-core.md §19, the CLI spike's `samples/` for real streams).
use super::js;
use serde_json::Value;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// What an event is. Electron Grace compared `type` strings; the kinds Grace reads are variants, and any other
/// (Karate's `ERROR`, a kind a later Karate adds) keeps its name in `Other`.
// Rust-native: a typed kind in place of `type: string`. Parity: `as_str` gives Karate's own name back, which the
// `events` tests check (`returns_only_complete_lines_and_keeps_a_partial_last_line`, `skips_blank_lines_...`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    SuiteEnter,
    SuiteExit,
    FeatureEnter,
    FeatureExit,
    OutlineEnter,
    ScenarioEnter,
    ScenarioExit,
    Error,
    Other(String),
}

impl EventKind {
    pub fn from_name(name: &str) -> Self {
        match name {
            "SUITE_ENTER" => Self::SuiteEnter,
            "SUITE_EXIT" => Self::SuiteExit,
            "FEATURE_ENTER" => Self::FeatureEnter,
            "FEATURE_EXIT" => Self::FeatureExit,
            "OUTLINE_ENTER" => Self::OutlineEnter,
            "SCENARIO_ENTER" => Self::ScenarioEnter,
            "SCENARIO_EXIT" => Self::ScenarioExit,
            "ERROR" => Self::Error,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Karate's name for the kind, as in the file.
    pub fn as_str(&self) -> &str {
        match self {
            Self::SuiteEnter => "SUITE_ENTER",
            Self::SuiteExit => "SUITE_EXIT",
            Self::FeatureEnter => "FEATURE_ENTER",
            Self::FeatureExit => "FEATURE_EXIT",
            Self::OutlineEnter => "OUTLINE_ENTER",
            Self::ScenarioEnter => "SCENARIO_ENTER",
            Self::ScenarioExit => "SCENARIO_EXIT",
            Self::Error => "ERROR",
            Self::Other(name) => name,
        }
    }
}

/// One line of `karate-events.jsonl`. `data` is Karate's own JSON, as written: its format is not a contract, so the
/// modules that read it take each field as it comes and tolerate what is missing.
#[derive(Clone, Debug, PartialEq)]
pub struct KarateEvent {
    pub kind: EventKind,
    pub time_stamp: f64,
    pub thread_id: Option<String>,
    /// Always a JSON object.
    pub data: Value,
}

impl KarateEvent {
    /// A line of the file as an event: a JSON object with a string `type` and an object `data`. `None` for anything
    /// else (`timeStamp` and `threadId` that are missing or of another type read as 0 and `None`: nothing reads them).
    pub fn parse(line: &str) -> Option<Self> {
        let Value::Object(mut map) = serde_json::from_str(line).ok()? else {
            return None;
        };
        let kind = EventKind::from_name(map.get("type")?.as_str()?);
        let data = map.remove("data").filter(Value::is_object)?;
        Some(Self {
            kind,
            time_stamp: map.get("timeStamp").and_then(Value::as_f64).unwrap_or(0.0),
            thread_id: map.get("threadId").and_then(Value::as_str).map(str::to_owned),
            data,
        })
    }

    /// `data.callDepth === 0`: the run's own features and scenarios, not the ones they call (AGENTS.md: filter
    /// `callDepth == 0` for the test tree).
    pub fn at_depth_zero(&self) -> bool {
        self.data.get("callDepth").and_then(Value::as_f64) == Some(0.0)
    }
}

/// Karate's event data was not shaped as Grace reads it. Electron Grace met these as `TypeError`s thrown by a property
/// read; they end a run the same way: "A bad event line or malformed data is fatal: the run's status can no longer be
/// trusted."
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataError(String);

impl DataError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DataError {}

/// Why a read of the events file failed.
#[derive(Debug)]
pub enum TailError {
    /// The file could not be read (other than: it does not exist yet).
    Io { file: PathBuf, source: std::io::Error },
    /// A line is not an event. Its text is not quoted: it can hold part of a secret.
    InvalidEvent { file: PathBuf, line: usize },
}

// The message of `Io` is the system's, so it reads differently from Node's (`EACCES: ...`); `InvalidEvent`'s is
// Electron Grace's, byte for byte: the run shows it in a notice.
impl fmt::Display for TailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { file, source } => write!(f, "{}: {source}", file.display()),
            Self::InvalidEvent { file, line } => {
                write!(f, "{}:{line}: invalid JSON event", file.display())
            }
        }
    }
}

impl std::error::Error for TailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidEvent { .. } => None,
        }
    }
}

/// Reads new complete lines of a growing JSONL file. A partial last line waits for its newline.
pub struct JsonlTail {
    file: PathBuf,
    offset: u64,
    line_no: usize,
}

impl JsonlTail {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            offset: 0,
            line_no: 0,
        }
    }

    // Rust-native: a `Result` in place of a thrown `Error`, and an event is checked when it is read (an object with a
    // string `type` and an object `data`), where Electron Grace kept any JSON and failed on its first property read.
    // Parity: the message of a bad line is the same, byte for byte (`does_not_quote_the_bad_line_...`,
    // `a_line_that_is_json_but_not_an_event_is_a_bad_line`).
    pub fn read(&mut self) -> Result<Vec<KarateEvent>, TailError> {
        let io = |source| TailError::Io {
            file: self.file.clone(),
            source,
        };
        let mut file = match File::open(&self.file) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(io(err)),
        };
        let size = file.metadata().map_err(io)?.len();
        let mut buf = Vec::new();
        if size > self.offset {
            file.seek(SeekFrom::Start(self.offset)).map_err(io)?;
            file.take(size - self.offset).read_to_end(&mut buf).map_err(io)?;
        }
        // Cut at the last newline on bytes, so a split multi-byte character is never decoded early.
        let Some(end) = buf.iter().rposition(|b| *b == b'\n') else {
            return Ok(Vec::new());
        };
        self.offset += end as u64 + 1;
        let mut events = Vec::new();
        for raw in String::from_utf8_lossy(&buf[..end]).split('\n') {
            self.line_no += 1;
            let text = js::trim(raw);
            if text.is_empty() {
                continue;
            }
            match KarateEvent::parse(text) {
                Some(event) => events.push(event),
                // not the parser's message: it quotes part of the line, which can hold a secret
                None => {
                    return Err(TailError::InvalidEvent {
                        file: self.file.clone(),
                        line: self.line_no,
                    });
                }
            }
        }
        Ok(events)
    }
}

pub fn read_all_events(file: impl AsRef<Path>) -> Result<Vec<KarateEvent>, TailError> {
    JsonlTail::new(file.as_ref()).read()
}

/// How often `Tailer` looks at the file during a run: Electron Grace's `pollMs`.
pub const POLL: Duration = Duration::from_millis(100);

/// What a `Tailer` hands over: the events that came, or the failure that ended it.
#[derive(Debug)]
pub enum TailMessage {
    /// New events, in file order. Never empty.
    Events(Vec<KarateEvent>),
    /// A read failed or a line was not an event. The tail has ended: the run's status can no longer be trusted, and
    /// the caller ends the run (Electron Grace's `fatal`).
    Failed(TailError),
}

enum Signal {
    Pause,
    Resume,
    Stop,
}

/// Follows a run's events file on its own thread, for as long as the run goes, and sends what it reads over a channel.
///
/// Electron Grace polled with `setInterval(poll, 100)` from its main thread, and stopped it around a debug pause
/// (R2.4). Here the thread waits on its control channel with the poll interval as the timeout: it wakes every `every`
/// while the run goes, never while paused, and ends with `stop` (or when the `Tailer` is dropped), so nothing outlives
/// the run (ADR 0030).
// Rust-native: a thread and a channel in place of a timer on the main thread. Parity: the same reads in the same
// order (`JsonlTail`, whose tests are the TypeScript's); the `tailer` tests here add the thread's own rules.
pub struct Tailer {
    control: Sender<Signal>,
    messages: Receiver<TailMessage>,
    thread: Option<JoinHandle<()>>,
}

impl Tailer {
    /// Starts following `file`, which need not exist yet, reading it every `every` (use `POLL`).
    pub fn start(file: impl Into<PathBuf>, every: Duration) -> std::io::Result<Self> {
        let (control, signals) = mpsc::channel();
        let (out, messages) = mpsc::channel();
        let tail = JsonlTail::new(file);
        let thread = thread::Builder::new()
            .name("karate-events".into())
            .spawn(move || follow(tail, every, &signals, &out))?;
        Ok(Self {
            control,
            messages,
            thread: Some(thread),
        })
    }

    /// What the thread has read, in order. A `Failed` is the last message.
    pub fn messages(&self) -> &Receiver<TailMessage> {
        &self.messages
    }

    /// Reads once more, then waits without waking until `resume`: nothing runs while a debug run is paused.
    pub fn pause(&self) {
        let _ = self.control.send(Signal::Pause);
    }

    /// Follows the file again, the next read an interval from now.
    pub fn resume(&self) {
        let _ = self.control.send(Signal::Resume);
    }

    /// Ends the run's tail: one last read, then the thread ends and is joined. Returns what the thread had read and
    /// the caller had not taken, that last read included (Electron Grace's `stopPoll(); poll();` after Java exits).
    pub fn stop(mut self) -> Vec<TailMessage> {
        self.end();
        self.messages.try_iter().collect()
    }

    fn end(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = self.control.send(Signal::Stop);
            let _ = thread.join();
        }
    }
}

impl Drop for Tailer {
    fn drop(&mut self) {
        self.end();
    }
}

fn follow(mut tail: JsonlTail, every: Duration, signals: &Receiver<Signal>, out: &Sender<TailMessage>) {
    // Reads once and sends what came. `false`: stop, because the read failed (as Electron Grace's `poll` stops after
    // its `fatal`) or nobody listens.
    let mut poll = || match tail.read() {
        Ok(events) if events.is_empty() => true,
        Ok(events) => out.send(TailMessage::Events(events)).is_ok(),
        Err(err) => {
            let _ = out.send(TailMessage::Failed(err));
            false
        }
    };
    let mut paused = false;
    loop {
        let signal = if paused {
            match signals.recv() {
                Ok(signal) => signal,
                Err(_) => return,
            }
        } else {
            match signals.recv_timeout(every) {
                Ok(signal) => signal,
                Err(RecvTimeoutError::Timeout) => {
                    if !poll() {
                        return;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        };
        match signal {
            Signal::Pause => {
                paused = true;
                if !poll() {
                    return;
                }
            }
            Signal::Resume => paused = false,
            Signal::Stop => {
                poll();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/events")
            .join(name)
    }

    fn line(kind: &str) -> String {
        format!(r#"{{"type":"{kind}","timeStamp":1,"threadId":null,"data":{{}}}}"#)
    }

    fn kinds(events: &[KarateEvent]) -> Vec<&str> {
        events.iter().map(|e| e.kind.as_str()).collect()
    }

    fn append(file: &Path, bytes: &[u8]) {
        let mut f = fs::OpenOptions::new().append(true).create(true).open(file).unwrap();
        f.write_all(bytes).unwrap();
    }

    mod jsonl_tail {
        use super::*;

        fn setup() -> (TempDir, PathBuf) {
            let dir = tempfile::Builder::new().prefix("jsonl-tail-").tempdir().unwrap();
            let file = dir.path().join("events.jsonl");
            (dir, file)
        }

        #[test]
        fn returns_empty_before_the_file_exists() {
            let (_dir, file) = setup();
            assert!(JsonlTail::new(&file).read().unwrap().is_empty());
            assert!(read_all_events(&file).unwrap().is_empty());
        }

        #[test]
        fn returns_only_complete_lines_and_keeps_a_partial_last_line() {
            let (_dir, file) = setup();
            let mut tail = JsonlTail::new(&file);
            let half = line("SCENARIO_ENTER");
            fs::write(
                &file,
                format!("{}\n{}\n{}", line("SUITE_ENTER"), line("FEATURE_ENTER"), &half[..20]),
            )
            .unwrap();
            assert_eq!(kinds(&tail.read().unwrap()), ["SUITE_ENTER", "FEATURE_ENTER"]);
            append(&file, format!("{}\n", &half[20..]).as_bytes());
            assert_eq!(kinds(&tail.read().unwrap()), ["SCENARIO_ENTER"]);
            assert!(tail.read().unwrap().is_empty());
        }

        #[test]
        fn does_not_decode_a_split_multi_byte_character_early() {
            let (_dir, file) = setup();
            let mut tail = JsonlTail::new(&file);
            let bytes = format!(
                "{}\n",
                r#"{"type":"X","timeStamp":1,"threadId":null,"data":{"name":"équipe"}}"#
            )
            .into_bytes();
            let cut = bytes.iter().position(|b| *b == 0xc3).unwrap() + 1; // between the two bytes of "é"
            fs::write(&file, &bytes[..cut]).unwrap();
            assert!(tail.read().unwrap().is_empty());
            append(&file, &bytes[cut..]);
            assert_eq!(tail.read().unwrap()[0].data["name"], "équipe");
        }

        #[test]
        fn skips_blank_lines_and_names_file_and_line_on_bad_json() {
            let (_dir, file) = setup();
            fs::write(&file, format!("{}\n\n{{oops\n", line("A"))).unwrap();
            let err = read_all_events(&file).unwrap_err();
            assert!(err.to_string().contains(&format!("{}:3", file.display())), "{err}");
            fs::write(&file, format!("{}\n\r\n{}\r\n", line("A"), line("B"))).unwrap();
            assert_eq!(kinds(&read_all_events(&file).unwrap()), ["A", "B"]);
        }

        #[test]
        fn does_not_quote_the_bad_line_it_can_hold_part_of_a_secret() {
            let (_dir, file) = setup();
            fs::write(&file, "{\"data\": s3cret-token-123}\n").unwrap();
            assert_eq!(
                read_all_events(&file).unwrap_err().to_string(),
                format!("{}:1: invalid JSON event", file.display())
            );
        }

        // New in the port: a line that is JSON but not an event reads as a bad line (Electron Grace kept it and
        // failed later, on its first property read), and a missing `timeStamp` or `threadId` is not an error.
        #[test]
        fn a_line_that_is_json_but_not_an_event_is_a_bad_line() {
            let (_dir, file) = setup();
            for bad in [
                "42",
                "null",
                r#"{"type":"A"}"#,
                r#"{"data":{}}"#,
                r#"{"type":"A","data":1}"#,
            ] {
                fs::write(&file, format!("{bad}\n")).unwrap();
                assert!(read_all_events(&file).is_err(), "{bad}");
            }
            fs::write(&file, "{\"type\":\"A\",\"data\":{}}\n").unwrap();
            let events = read_all_events(&file).unwrap();
            assert_eq!(events[0].time_stamp, 0.0);
            assert_eq!(events[0].thread_id, None);
        }
    }

    mod read_all_events {
        use super::*;

        #[test]
        fn parses_a_real_event_stream() {
            let events = read_all_events(fixture("run-clean.jsonl")).unwrap();
            let count = |kind: EventKind| events.iter().filter(|e| e.kind == kind).count();
            assert_eq!(count(EventKind::SuiteEnter), 1);
            assert_eq!(count(EventKind::SuiteExit), 1);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.kind == EventKind::ScenarioExit && e.at_depth_zero())
                    .count(),
                9
            );
        }

        #[test]
        fn handles_a_file_that_ends_without_exit_events() {
            let events = read_all_events(fixture("4-after-taskkill-complete-file.jsonl")).unwrap();
            assert_eq!(events.last().map(|e| &e.kind), Some(&EventKind::ScenarioEnter));
        }
    }

    // New in the port: `Tailer` replaces the TypeScript's `setInterval` poll in `runner.ts`.
    mod tailer {
        use super::*;

        const SOON: Duration = Duration::from_secs(10);
        /// An interval no test waits out: only a signal makes the thread read.
        const NEVER: Duration = Duration::from_secs(3600);

        fn next(tailer: &Tailer) -> TailMessage {
            tailer.messages().recv_timeout(SOON).unwrap()
        }

        #[test]
        fn sends_the_events_that_grow_while_it_runs() {
            let dir = TempDir::new().unwrap();
            let file = dir.path().join("events.jsonl");
            let tailer = Tailer::start(&file, Duration::from_millis(5)).unwrap();
            append(&file, format!("{}\n", line("SUITE_ENTER")).as_bytes());
            let TailMessage::Events(first) = next(&tailer) else {
                panic!("not events")
            };
            assert_eq!(kinds(&first), ["SUITE_ENTER"]);
            append(&file, format!("{}\n{}\n", line("A"), line("B")).as_bytes());
            let TailMessage::Events(second) = next(&tailer) else {
                panic!("not events")
            };
            assert_eq!(kinds(&second), ["A", "B"]);
            assert!(tailer.stop().is_empty());
        }

        #[test]
        fn stop_reads_once_more_and_does_not_poll_before() {
            let dir = TempDir::new().unwrap();
            let file = dir.path().join("events.jsonl");
            let tailer = Tailer::start(&file, NEVER).unwrap();
            append(&file, format!("{}\n", line("SUITE_EXIT")).as_bytes());
            assert!(tailer.messages().try_recv().is_err());
            let rest = tailer.stop();
            assert_eq!(rest.len(), 1);
            let TailMessage::Events(events) = &rest[0] else {
                panic!("not events")
            };
            assert_eq!(kinds(events), ["SUITE_EXIT"]);
        }

        #[test]
        fn a_bad_line_ends_the_tail_with_a_failure() {
            let dir = TempDir::new().unwrap();
            let file = dir.path().join("events.jsonl");
            append(&file, b"{oops\n");
            let tailer = Tailer::start(&file, Duration::from_millis(5)).unwrap();
            let TailMessage::Failed(err) = next(&tailer) else {
                panic!("not a failure")
            };
            assert_eq!(err.to_string(), format!("{}:1: invalid JSON event", file.display()));
            append(&file, format!("{}\n", line("A")).as_bytes());
            assert!(tailer.stop().is_empty(), "nothing is read after a failure");
        }

        #[test]
        fn pause_reads_once_and_then_waits_until_resume() {
            let dir = TempDir::new().unwrap();
            let file = dir.path().join("events.jsonl");
            let tailer = Tailer::start(&file, NEVER).unwrap();
            append(&file, format!("{}\n", line("A")).as_bytes());
            tailer.pause();
            let TailMessage::Events(a) = next(&tailer) else {
                panic!("not events")
            };
            assert_eq!(kinds(&a), ["A"]);
            append(&file, format!("{}\n", line("B")).as_bytes());
            assert!(
                tailer.messages().recv_timeout(Duration::from_millis(50)).is_err(),
                "a paused tail does not read"
            );
            tailer.resume();
            let rest = tailer.stop();
            let TailMessage::Events(b) = &rest[0] else {
                panic!("not events")
            };
            assert_eq!(kinds(b), ["B"]);
        }

        #[test]
        fn dropping_it_ends_the_thread() {
            let dir = TempDir::new().unwrap();
            let tailer = Tailer::start(dir.path().join("events.jsonl"), NEVER).unwrap();
            drop(tailer); // joins: the test hangs here if the thread did not end
        }
    }
}
