//! `src/main/karate/process.ts`: the Java of a dry run or a run, and a project's build tool. Started without a shell,
//! in the project folder, in a cleaned environment, below normal priority; its output read line by line; cancelled
//! with everything it started; and the Java a crash of Grace left behind, killed at the next start.
//!
//! What differs by platform is one function each in `sys` (migration guide R5.3), with a test on each platform:
//!
//! | Seam | Windows | Linux |
//! |---|---|---|
//! | `sys::prepare`: priority | `BELOW_NORMAL_PRIORITY_CLASS` (#148), no console window | nice 10, its own process group, `LC_ALL=C.UTF-8` |
//! | `sys::contain`: dies with Grace | a job object that kills it when Grace ends | nothing |
//! | `sys::kill_tree`: cancel | `taskkill /PID <pid> /T /F` | `SIGKILL` to its process group |
//! | `sys::executable`: whose is the orphan | `QueryFullProcessImageNameW` | `/proc/<pid>/exe` |
//! | `sys::add_args`: a build tool's command line | as written (`cmd.exe /d /s /c "…"`) | always as given |
use super::command::{Env, child_env};
use crate::model::js;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

/// How a process ended: its exit code, or None when it could not start or a signal ended it (Linux). Windows reads
/// `taskkill /F` as exit code 1.
pub type Exit = Option<i32>;

/// What `spawn_karate` may change for a build tool, and how long `kill` waits.
#[derive(Clone, Debug)]
pub struct SpawnOptions {
    /// How long `kill` waits for the process to end, twice: after the tree kill, then after killing the process alone.
    pub kill_wait: Duration,
    /// A build tool's own environment. None gives Grace's own, cleaned (`command::child_env`).
    pub env: Option<Env>,
    /// Windows: the arguments as written, unquoted, as a build tool's `cmd.exe /d /s /c "…"` needs. Linux has no
    /// command line to quote: the arguments always reach the program as given.
    pub verbatim_args: bool,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            kill_wait: Duration::from_secs(5),
            env: None,
            verbatim_args: false,
        }
    }
}

struct Shared {
    exit: Mutex<Option<Exit>>,
    ended: Condvar,
    /// The child until its output ends; then the thread that read it waits for it, and `kill` has the tree kill only.
    child: Mutex<Option<Child>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A started process. `Send` and `Sync`: one thread may wait for it while another kills it.
pub struct KarateProcess {
    pid: Option<u32>,
    shared: Arc<Shared>,
    kill_wait: Duration,
}

impl KarateProcess {
    /// A process that did not start: it has ended, without an exit code.
    fn not_started(kill_wait: Duration) -> Self {
        Self {
            pid: None,
            shared: Arc::new(Shared {
                exit: Mutex::new(Some(None)),
                ended: Condvar::new(),
                child: Mutex::new(None),
            }),
            kill_wait,
        }
    }

    /// The process id; None when it did not start (Electron Grace's `-1`).
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// How it ended, if it has: the process has exited and every line of its output has been handed over.
    pub fn exit(&self) -> Option<Exit> {
        *lock(&self.shared.exit)
    }

    /// Waits until it has ended (Electron Grace's `await exit`).
    pub fn wait(&self) -> Exit {
        let mut exit = lock(&self.shared.exit);
        loop {
            if let Some(code) = *exit {
                return code;
            }
            exit = self.shared.ended.wait(exit).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Waits at most `timeout`: how it ended, or None if it has not (Electron Grace's `within(exit, ms)`).
    pub fn wait_timeout(&self, timeout: Duration) -> Option<Exit> {
        let exit = lock(&self.shared.exit);
        let (exit, _) = self
            .shared
            .ended
            .wait_timeout_while(exit, timeout, |e| e.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        *exit
    }

    /// Kills the process and everything it started (`taskkill /PID <pid> /T /F` on Windows, its process group on
    /// Linux), then waits up to `kill_wait` for it to end; if it has not, kills the process alone and waits once more.
    /// Returns when it has ended or both waits are over, so quitting never hangs. Does nothing once it has ended.
    pub fn kill(&self) {
        if self.exit().is_some() {
            return;
        }
        if let Some(pid) = self.pid {
            sys::kill_tree(pid);
        }
        if self.wait_timeout(self.kill_wait).is_some() {
            return;
        }
        if let Some(child) = lock(&self.shared.child).as_mut() {
            let _ = child.kill();
        }
        self.wait_timeout(self.kill_wait);
    }
}

/// Starts `program` with `args` in `cwd`, never through a shell, and hands each line it prints (standard output and
/// error, as they come) to `on_lines`, from a thread of its own that lives as long as the output does. `on_lines`
/// gets the lines of each read of the output, at least one; the last call comes before `wait` returns.
///
/// When the program cannot start, `on_lines` gets one line, `spawn <program> <code>` as Node writes it (`ENOENT` for a
/// missing program), on the calling thread, and the process has ended without an exit code.
// Rust-native: one pipe for standard output and error, read by one thread that hands over the lines of each read and
// then waits for the process, in place of two readline interfaces, a 100 ms flush timer and a `close` event. The
// lines come as soon as Java prints them and nothing wakes on a timer (ADR 0030); a test of Electron Grace's that
// waits on `exit` sees every line first, as before. Parity: `runs_karate_and_streams_console_lines`,
// `reports_a_missing_java_as_one_line_and_a_null_exit`, and `lines::splits_lines_as_nodes_readline_does` for the
// line ends.
pub fn spawn_karate<S: AsRef<OsStr>>(
    program: &Path,
    args: &[S],
    cwd: &Path,
    mut on_lines: impl FnMut(Vec<String>) + Send + 'static,
    options: SpawnOptions,
) -> KarateProcess {
    let SpawnOptions {
        kill_wait,
        env,
        verbatim_args,
    } = options;
    let env = env.unwrap_or_else(|| child_env(std::env::vars_os(), &[]));
    let (child, output) = match start(program, args, cwd, env, verbatim_args) {
        Ok(started) => started,
        Err(err) => {
            on_lines(vec![spawn_error(program, &err)]);
            return KarateProcess::not_started(kill_wait);
        }
    };
    let pid = child.id();
    let shared = Arc::new(Shared {
        exit: Mutex::new(None),
        ended: Condvar::new(),
        child: Mutex::new(Some(child)),
    });
    let reader = Arc::clone(&shared);
    let spawned = thread::Builder::new()
        .name("karate-output".into())
        .spawn(move || read_output(output, on_lines, &reader));
    if spawned.is_err() {
        // no thread to read its output: end it here, as if it had not started
        sys::kill_tree(pid);
        if let Some(mut child) = lock(&shared.child).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *lock(&shared.exit) = Some(None);
    }
    KarateProcess {
        pid: Some(pid),
        shared,
        kill_wait,
    }
}

/// Spawns the child with one pipe for its standard output and error, and its standard input the null device.
fn start<S: AsRef<OsStr>>(
    program: &Path,
    args: &[S],
    cwd: &Path,
    env: Env,
    verbatim_args: bool,
) -> io::Result<(Child, io::PipeReader)> {
    let (output, input) = io::pipe()?;
    let mut cmd = Command::new(program);
    sys::add_args(&mut cmd, args, verbatim_args);
    cmd.current_dir(cwd)
        .env_clear()
        .envs(env)
        .stdin(Stdio::null())
        .stdout(input.try_clone()?)
        .stderr(input);
    sys::prepare(&mut cmd);
    let child = cmd.spawn()?;
    // closes this side's copies of the pipe's input: the output ends when the child's last copy closes
    drop(cmd);
    sys::contain(&child);
    Ok((child, output))
}

/// Node's message for a process that could not start: `spawn <file> <code>`.
fn spawn_error(program: &Path, err: &io::Error) -> String {
    let code = match err.kind() {
        io::ErrorKind::NotFound => "ENOENT".to_string(),
        io::ErrorKind::PermissionDenied => "EACCES".to_string(),
        _ => err.to_string(),
    };
    format!("spawn {} {code}", program.display())
}

/// The output thread: hands over the lines of each read, the last one without its line end, then waits for the
/// process and records how it ended.
fn read_output(mut output: io::PipeReader, mut on_lines: impl FnMut(Vec<String>), shared: &Shared) {
    let mut lines = LineSplitter::default();
    let mut buf = [0u8; 16 * 1024];
    loop {
        match output.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let read = lines.push(&buf[..n]);
                if !read.is_empty() {
                    on_lines(read);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    if let Some(last) = lines.finish() {
        on_lines(vec![last]);
    }
    drop(output);
    let child = lock(&shared.child).take();
    let exit = child.and_then(|mut c| c.wait().ok()).and_then(|status| status.code());
    *lock(&shared.exit) = Some(exit);
    shared.ended.notify_all();
}

/// Node's `readline`: a line ends at `\n`, `\r\n` or a lone `\r`, and a last line without an end is a line too. Bytes
/// that are not UTF-8 read as U+FFFD, as Node's decoder reads them.
///
/// Node drops the `\n` of a `\r\n` only when it comes within 100 ms of the `\r` (`crlfDelay`); here it is always
/// dropped, wherever the two reads split.
#[derive(Default)]
struct LineSplitter {
    line: Vec<u8>,
    after_cr: bool,
}

impl LineSplitter {
    /// The lines `bytes` ends.
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for &b in bytes {
            if std::mem::take(&mut self.after_cr) && b == b'\n' {
                continue;
            }
            if b == b'\n' || b == b'\r' {
                lines.push(String::from_utf8_lossy(&self.line).into_owned());
                self.line.clear();
                self.after_cr = b == b'\r';
            } else {
                self.line.push(b);
            }
        }
        lines
    }

    /// The last line, if the output ended inside one.
    fn finish(self) -> Option<String> {
        (!self.line.is_empty()).then(|| String::from_utf8_lossy(&self.line).into_owned())
    }
}

/// Whether `exe` is a file under `dir`, both with their links resolved where they exist; without regard to case on
/// Windows.
fn runs_from(exe: &Path, dir: &Path) -> bool {
    let real = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| super::command::normalize(p));
    let text = |p: PathBuf| {
        let t = p.to_string_lossy().into_owned();
        if cfg!(windows) { t.to_lowercase() } else { t }
    };
    let (exe, dir) = (text(real(exe)), text(real(dir)));
    exe.strip_prefix(&dir)
        .is_some_and(|rest| rest.starts_with(MAIN_SEPARATOR))
}

/// Kills the tree of the process whose id is in `pid_file` only if it is a process running from inside `jre_dir`
/// (process ids are reused). Always deletes the file when there is one.
// Rust-native: the executable of the process comes from the system (`QueryFullProcessImageNameW`, `/proc/<pid>/exe`)
// in place of a PowerShell `Get-CimInstance` query, which took a second to start at Grace's own start. Parity:
// `kills_only_our_java`, `ignores_a_missing_or_garbage_pid_file`.
pub fn kill_orphan(pid_file: &Path, jre_dir: &Path) {
    let Ok(bytes) = fs::read(pid_file) else {
        return; // no file: nothing to do
    };
    let text = String::from_utf8_lossy(&bytes);
    let text = js::trim(&text);
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        let ours = text
            .parse::<u32>()
            .ok()
            .filter(|pid| sys::executable(*pid).is_some_and(|exe| runs_from(&exe, jre_dir)));
        if let Some(pid) = ours {
            sys::kill_tree(pid);
        }
    }
    let _ = fs::remove_file(pid_file);
}

/// `cmd.exe` by its full path, which runs a project's build tool: its wrappers are batch files, which only cmd.exe
/// runs. A `cmd.exe` earlier on PATH is never run instead.
#[cfg(windows)]
pub fn cmd() -> PathBuf {
    sys::system32().join("cmd.exe")
}

#[cfg(windows)]
mod sys {
    use super::*;
    use std::ffi::{OsString, c_void};
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::sync::OnceLock;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_BREAKAWAY_OK,
        JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    use windows::Win32::System::Threading::{
        BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW, OpenProcess, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    use windows::core::{PCWSTR, PWSTR};

    /// `%SystemRoot%\System32`: a taskkill.exe or cmd.exe earlier on PATH must not be run instead.
    pub fn system32() -> PathBuf {
        let root = std::env::var_os("SystemRoot")
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32")
    }

    /// Node's `windowsVerbatimArguments`: each argument added to the command line as written.
    pub fn add_args<S: AsRef<OsStr>>(cmd: &mut Command, args: &[S], verbatim: bool) {
        for arg in args {
            if verbatim {
                cmd.raw_arg(arg);
            } else {
                cmd.arg(arg);
            }
        }
    }

    /// Java gets only the CPU time nothing else wants, so a run never makes the IDE stutter (#148); set when the
    /// process is created. No console window (Node's `windowsHide`).
    pub fn prepare(cmd: &mut Command) {
        cmd.creation_flags(CREATE_NO_WINDOW.0 | BELOW_NORMAL_PRIORITY_CLASS.0);
    }

    /// Grace's job object, made at the first spawn and held until Grace ends: closing its last handle, which Windows
    /// does when Grace's process ends however it ends, kills every process still in it. A process in it may leave
    /// it when it asks to (`CREATE_BREAKAWAY_FROM_JOB`, as the updater's switch script must, ADR 0019), and one that
    /// crashes ends without the crash dialog.
    pub(super) fn job() -> Option<HANDLE> {
        static JOB: OnceLock<Option<usize>> = OnceLock::new();
        let job = JOB.get_or_init(|| {
            // SAFETY: plain Win32 calls on a handle made here; `info` lives across the call that reads it.
            unsafe {
                let job = CreateJobObjectW(None, PCWSTR::null()).ok()?;
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_BREAKAWAY_OK
                    | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
                let set = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast::<c_void>(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if set.is_err() {
                    let _ = CloseHandle(job);
                    return None;
                }
                Some(job.0 as usize)
            }
        });
        job.map(|h| HANDLE(h as *mut c_void))
    }

    /// Puts the child, and so everything it starts from now on, in Grace's job. Right after the spawn: a process
    /// the child started in the microseconds between would be left out, and Java starts none that early.
    pub fn contain(child: &Child) {
        if let Some(job) = job() {
            // SAFETY: both handles are open: the job's for Grace's life, the child's for the `Child`'s.
            let _ = unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())) };
        }
    }

    /// `taskkill /PID <pid> /T /F`, waited for, by its full path.
    pub fn kill_tree(pid: u32) {
        let _ = Command::new(system32().join("taskkill.exe"))
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    /// The executable a process runs, when Grace may ask.
    pub fn executable(pid: u32) -> Option<PathBuf> {
        let mut buf = vec![0u16; 32_768];
        let mut len = buf.len() as u32;
        // SAFETY: the process handle is closed before returning; `buf` holds `len` units, as the call is told.
        let read = unsafe {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
            let _ = CloseHandle(process);
            read
        };
        read.ok()?;
        Some(PathBuf::from(OsString::from_wide(&buf[..len as usize])))
    }
}

#[cfg(unix)]
mod sys {
    use super::*;
    use std::os::unix::process::CommandExt;

    /// Linux's below-normal priority (migration guide R5.3).
    const NICE: libc::c_int = 10;

    /// The arguments reach the program as given: there is no command line to write them in.
    pub fn add_args<S: AsRef<OsStr>>(cmd: &mut Command, args: &[S], _verbatim: bool) {
        cmd.args(args);
    }

    /// Java runs at nice 10, so a run never makes the IDE stutter (#148), set before it starts so each of its threads
    /// has it; leads its own process group, which `kill_tree` ends; and reads its command line as UTF-8: under
    /// `LANG=C` (a CI container) Java reads a path with `é` as "malformed input". Windows hands Java its command line
    /// in UTF-16 and needs none of this.
    pub fn prepare(cmd: &mut Command) {
        cmd.process_group(0).env("LC_ALL", "C.UTF-8");
        // SAFETY: the hook runs in the child between fork and exec, and makes one system call, which is safe there.
        // A priority it may not set (Grace itself at a higher nice) leaves the child at Grace's.
        unsafe {
            cmd.pre_exec(|| {
                libc::setpriority(libc::PRIO_PROCESS, 0, NICE);
                Ok(())
            });
        }
    }

    /// Nothing: Linux has no job object (plan P1, porting rule 5). A run is still cancelled whole by its group.
    pub fn contain(_child: &Child) {}

    /// `SIGKILL` to the process group the process leads, else to the process alone. Never to group 0 or 1 (Grace's
    /// own group, and every process).
    pub fn kill_tree(pid: u32) {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return;
        };
        if pid <= 1 {
            return;
        }
        // SAFETY: `kill` takes any process or group id; a wrong one fails with ESRCH or EPERM.
        unsafe {
            if libc::kill(-pid, libc::SIGKILL) != 0 {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }

    /// The executable a process runs, links resolved.
    pub fn executable(pid: u32) -> Option<PathBuf> {
        fs::read_link(format!("/proc/{pid}/exe")).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::command::{BuildArgs, build_args, detect_layout};
    use crate::karate::testing::{copy_fixture, vendor};
    use std::time::Instant;
    use tempfile::TempDir;

    type Lines = Arc<Mutex<Vec<String>>>;

    /// `(l) => lines.push(...l)`: the lines, and the callback that collects them.
    fn sink() -> (Lines, impl FnMut(Vec<String>) + Send + 'static) {
        let lines = Lines::default();
        let collect = Arc::clone(&lines);
        (lines, move |l: Vec<String>| collect.lock().unwrap().extend(l))
    }

    fn sleep(ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }

    fn tmp(prefix: &str) -> TempDir {
        tempfile::Builder::new().prefix(prefix).tempdir().unwrap()
    }

    /// A started process, killed when the test ends however it ends, as the TypeScript's `afterEach` does; then its
    /// folders are removed.
    struct Started {
        p: KarateProcess,
        root: TempDir,
        _out: TempDir,
    }

    impl Drop for Started {
        fn drop(&mut self) {
            self.p.kill();
        }
    }

    /// Karate on a copy of the `clean` fixture, one thread: a dry run of everything, or a run of one feature.
    fn start(dry_run: bool, on_lines: impl FnMut(Vec<String>) + Send + 'static, feature: Option<&str>) -> Started {
        let vendor = vendor();
        let root = copy_fixture("clean");
        let out = tmp("kk-out-");
        let layout = detect_layout(root.path());
        let targets: Vec<String> = match feature {
            Some(f) => vec![
                f.split('/')
                    .fold(root.path().to_path_buf(), |p, s| p.join(s))
                    .to_string_lossy()
                    .into_owned(),
            ],
            None => layout
                .feature_roots
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
        };
        let args = build_args(&BuildArgs {
            jar: &vendor.jar,
            classpath: &layout.classpath,
            root: root.path(),
            out_dir: out.path(),
            dry_run,
            tags: &[],
            targets: &targets,
            env: None,
            arg_file: None,
            threads: 1,
            config_dir: None,
            debug: None,
            junit_xml: false,
        });
        let p = spawn_karate(&vendor.java, &args, root.path(), on_lines, SpawnOptions::default());
        Started { p, root, _out: out }
    }

    /// Kills a process tree when the test ends, as the TypeScript's `finally` does.
    struct KillOnDrop(u32);

    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            sys::kill_tree(self.0);
        }
    }

    #[cfg(unix)]
    fn stat(pid: u32) -> Option<Vec<String>> {
        let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // after `pid (comm) `: the state is field 3, the process group 5, nice 19
        let rest = &text[text.rfind(')')? + 2..];
        Some(rest.split_whitespace().map(String::from).collect())
    }

    /// The process exists and has not exited (an exited process nobody has waited for yet is not alive).
    #[cfg(unix)]
    fn alive(pid: u32) -> bool {
        stat(pid).is_some_and(|f| f[0] != "Z")
    }

    #[cfg(windows)]
    fn alive(pid: u32) -> bool {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    /// The command lines of the running processes.
    #[cfg(unix)]
    fn command_lines() -> Vec<String> {
        fs::read_dir("/proc")
            .unwrap()
            .flatten()
            .filter_map(|e| fs::read(e.path().join("cmdline")).ok())
            .map(|b| String::from_utf8_lossy(&b).replace('\0', " "))
            .collect()
    }

    /// The command lines of the running `java.exe`s.
    #[cfg(windows)]
    fn command_lines() -> Vec<String> {
        let query = "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").CommandLine";
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", query])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).lines().map(String::from).collect()
    }

    /// `pidFile(pid)`: a `running.pid` holding `text`, in a folder of its own.
    fn pid_file(text: &str) -> (TempDir, PathBuf) {
        let dir = tmp("kk-pid-");
        let file = dir.path().join("running.pid");
        fs::write(&file, text).unwrap();
        (dir, file)
    }

    /// A program that runs for a minute.
    fn sleeper() -> (PathBuf, Vec<String>) {
        if cfg!(windows) {
            let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()))
                .join("System32")
                .join("PING.EXE");
            (ping, ["-n", "61", "127.0.0.1"].map(String::from).to_vec())
        } else {
            ("sleep".into(), vec!["60".into()])
        }
    }

    mod spawn_karate {
        use super::*;

        #[cfg(windows)]
        #[test]
        fn passes_the_given_environment_and_verbatim_arguments() {
            let (lines, on_lines) = sink();
            let env: Env = std::env::vars_os()
                .chain([("GRACE_X".into(), "a b&c".into())])
                .collect();
            let p = spawn_karate(
                &cmd(),
                &["/d", "/s", "/c", r#""echo "%GRACE_X%"""#],
                &std::env::temp_dir(),
                on_lines,
                SpawnOptions {
                    env: Some(env),
                    verbatim_args: true,
                    ..SpawnOptions::default()
                },
            );
            assert_eq!(p.wait(), Some(0));
            assert_eq!(*lines.lock().unwrap(), [r#""a b&c""#]);
        }

        // Linux has no command line to keep as written: the same environment, and the argument reaches `sh` as given.
        #[cfg(unix)]
        #[test]
        fn passes_the_given_environment_and_verbatim_arguments() {
            let (lines, on_lines) = sink();
            let env: Env = std::env::vars_os()
                .chain([("GRACE_X".into(), "a b&c".into())])
                .collect();
            let p = spawn_karate(
                Path::new("sh"),
                &["-c", r#"printf '%s\n' "\"$GRACE_X\"""#],
                &std::env::temp_dir(),
                on_lines,
                SpawnOptions {
                    env: Some(env),
                    verbatim_args: true,
                    ..SpawnOptions::default()
                },
            );
            assert_eq!(p.wait(), Some(0));
            assert_eq!(*lines.lock().unwrap(), [r#""a b&c""#]);
        }

        #[test]
        fn runs_karate_and_streams_console_lines() {
            let (lines, on_lines) = sink();
            let s = start(true, on_lines, None);
            let exit = s.p.wait();
            let lines = lines.lock().unwrap();
            assert_eq!(exit, Some(0), "{}", lines.join("\n"));
            assert!(lines.iter().any(|l| l.contains("DRY RUN")), "{}", lines.join("\n"));
        }

        #[test]
        fn reports_a_missing_java_as_one_line_and_a_null_exit() {
            let (lines, on_lines) = sink();
            let java = std::env::temp_dir().join("nope").join("java.exe");
            let p = spawn_karate(
                &java,
                &[] as &[&str],
                &std::env::temp_dir(),
                on_lines,
                SpawnOptions::default(),
            );
            assert_eq!(p.wait(), None);
            assert_eq!(p.pid(), None);
            let lines = lines.lock().unwrap();
            assert_eq!(lines.len(), 1);
            assert!(lines[0].contains("ENOENT"), "{}", lines[0]);
        }

        #[cfg(windows)]
        #[test]
        fn runs_java_below_normal_priority_148() {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::Threading::{
                BELOW_NORMAL_PRIORITY_CLASS, GetPriorityClass, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            };
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            // SAFETY: the handle is closed after the one call that reads it.
            let class = unsafe {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, s.p.pid().unwrap()).unwrap();
                let class = GetPriorityClass(process);
                let _ = CloseHandle(process);
                class
            };
            assert_eq!(class, BELOW_NORMAL_PRIORITY_CLASS.0);
        }

        #[cfg(unix)]
        #[test]
        fn runs_java_below_normal_priority_148() {
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            let fields = stat(s.p.pid().unwrap()).unwrap();
            assert_eq!(fields[16], "10", "nice");
        }

        #[test]
        fn kill_leaves_no_java_behind() {
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            sleep(1500);
            s.p.kill();
            s.p.wait();
            let pid = s.p.pid().unwrap();
            assert!(!alive(pid));
            // no java of this project may survive; other tests run their own java meanwhile
            let root = s.root.path().to_string_lossy().into_owned();
            assert!(!command_lines().iter().any(|c| c.contains(&root)));
            s.p.kill(); // already gone: must not panic
        }

        // Not in the TypeScript tests: the other half of each platform's "everything Java started" seam. On Windows
        // Java is in the job that kills it when Grace ends, however Grace ends; on Linux it leads its own process
        // group, which a cancel kills whole.
        #[cfg(windows)]
        #[test]
        fn puts_java_in_the_job_that_ends_with_grace() {
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::JobObjects::{
                IsProcessInJob, JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, QueryInformationJobObject,
            };
            use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
            use windows::core::BOOL;
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            let job = sys::job().expect("Grace's job");
            let mut in_job = BOOL(0);
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            // SAFETY: the process handle is closed after the calls that read it; `info` and `in_job` outlive them.
            unsafe {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, s.p.pid().unwrap()).unwrap();
                IsProcessInJob(process, Some(job), &mut in_job).unwrap();
                let _ = CloseHandle(process);
                QueryInformationJobObject(
                    Some(job),
                    JobObjectExtendedLimitInformation,
                    (&raw mut info).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    None,
                )
                .unwrap();
            }
            assert!(in_job.as_bool());
            let flags = info.BasicLimitInformation.LimitFlags;
            assert!(flags.contains(JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE));
            assert!(flags.contains(JOB_OBJECT_LIMIT_BREAKAWAY_OK));
        }

        #[cfg(unix)]
        #[test]
        fn puts_java_in_a_process_group_of_its_own() {
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            let pid = s.p.pid().unwrap();
            assert_eq!(stat(pid).unwrap()[2], pid.to_string(), "process group");
        }
    }

    mod spawn_karate_kill {
        use super::*;

        /// A program that starts a grandchild outside its tree (Linux: a session of its own; Windows: started by
        /// PowerShell, whose tree ends with it) that keeps the output open for a minute, prints its id, and exits.
        fn detached_grandchild() -> (PathBuf, Vec<String>) {
            if cfg!(windows) {
                let script = "(Start-Process -FilePath \"$env:SystemRoot\\System32\\PING.EXE\" \
                              -ArgumentList '-n','61','127.0.0.1' -NoNewWindow -PassThru).Id";
                let args = ["-NoProfile", "-NonInteractive", "-Command", script];
                ("powershell".into(), args.map(String::from).to_vec())
            } else {
                (
                    "sh".into(),
                    ["-c", "setsid sleep 60 & echo $!"].map(String::from).to_vec(),
                )
            }
        }

        #[test]
        fn gives_up_waiting_when_the_process_does_not_end() {
            // The child starts a detached grandchild that keeps its pipes open, then exits: the tree kill finds no
            // process and the exit cannot be known until the grandchild ends.
            let (lines, on_lines) = sink();
            let (program, args) = detached_grandchild();
            let p = spawn_karate(
                &program,
                &args,
                &std::env::temp_dir(),
                on_lines,
                SpawnOptions {
                    kill_wait: Duration::from_millis(200),
                    ..SpawnOptions::default()
                },
            );
            let pid = p.pid().unwrap();
            let grandchild = || lines.lock().unwrap().iter().find_map(|l| l.trim().parse::<u32>().ok());
            let end = Instant::now() + Duration::from_secs(20);
            while (grandchild().is_none() || alive(pid)) && Instant::now() < end {
                sleep(100);
            }
            let grandchild = grandchild().expect("the grandchild's id");
            {
                let _finally = KillOnDrop(grandchild);
                assert!(!alive(pid));
                let t0 = Instant::now();
                p.kill();
                assert!(t0.elapsed() < Duration::from_secs(10));
                // still open: kill() returned anyway
                assert_eq!(p.wait_timeout(Duration::from_millis(100)), None);
            }
            assert_eq!(p.wait(), Some(0));
        }
    }

    // Electron Grace's `within(promise, ms)` is a general helper; for a process it is `wait_timeout`, tested here.
    mod within {
        use super::*;

        #[test]
        fn tells_whether_a_promise_settles_in_time() {
            let (program, args): (PathBuf, &[&str]) = if cfg!(windows) {
                (cmd_exe(), &["/d", "/c", "exit 0"])
            } else {
                ("sh".into(), &["-c", "exit 0"])
            };
            let done = spawn_karate(&program, args, &std::env::temp_dir(), |_| {}, SpawnOptions::default());
            done.wait();
            assert_eq!(done.wait_timeout(Duration::from_millis(50)), Some(Some(0)));

            let (program, args) = sleeper();
            let never = Running(spawn_karate(
                &program,
                &args,
                &std::env::temp_dir(),
                |_| {},
                SpawnOptions::default(),
            ));
            assert_eq!(never.0.wait_timeout(Duration::from_millis(50)), None);
        }

        /// A process killed when the test ends, however it ends.
        struct Running(KarateProcess);

        impl Drop for Running {
            fn drop(&mut self) {
                self.0.kill();
            }
        }

        fn cmd_exe() -> PathBuf {
            PathBuf::from(std::env::var_os("SystemRoot").unwrap_or("C:\\Windows".into()))
                .join("System32")
                .join("cmd.exe")
        }
    }

    mod kill_orphan {
        use super::*;

        #[test]
        fn kills_only_our_java() {
            let vendor = vendor();
            let s = start(false, |_| {}, Some("tests/slow.feature"));
            sleep(1500);
            let pid = s.p.pid().unwrap();
            let (_dir, file) = pid_file(&pid.to_string());
            kill_orphan(&file, &vendor.jre_dir);
            assert_ne!(s.p.wait(), Some(0));
            assert!(!alive(pid));
            assert!(!file.exists());

            let me = std::process::id();
            let (_dir, other) = pid_file(&me.to_string());
            kill_orphan(&other, &vendor.jre_dir);
            assert!(alive(me));
            assert!(!other.exists());
        }

        #[test]
        fn ignores_a_missing_or_garbage_pid_file() {
            let jre_dir = vendor().jre_dir;
            kill_orphan(&std::env::temp_dir().join("kk-missing.pid"), &jre_dir);
            let (_dir, f) = pid_file("1; calc");
            kill_orphan(&f, &jre_dir);
            assert!(!f.exists());
        }
    }

    // Not in the TypeScript tests: the line ends of Node's `readline`, which Electron Grace got for free.
    mod lines {
        use super::*;

        #[test]
        fn splits_lines_as_nodes_readline_does() {
            let mut s = LineSplitter::default();
            assert_eq!(s.push(b"a\nb\r\nc\rd"), ["a", "b", "c"]);
            // a `\r` ends its line at once; the `\n` after it, read later, ends none
            assert_eq!(s.push(b"\r"), ["d"]);
            assert_eq!(s.push(b"\ne\n\n"), ["e", ""]);
            // a character split between two reads, and a byte that is not UTF-8
            assert_eq!(s.push(b"caf\xc3"), [] as [String; 0]);
            assert_eq!(s.push(b"\xa9 \xff\n"), ["café \u{fffd}"]);
            assert_eq!(s.push(b"last"), [] as [String; 0]);
            assert_eq!(s.finish().as_deref(), Some("last"));
            assert_eq!(LineSplitter::default().finish(), None);
        }
    }
}
