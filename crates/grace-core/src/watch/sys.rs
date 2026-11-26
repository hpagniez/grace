//! Windows' side of the watcher: every `unsafe` call of it. One thread per directory handle reads
//! `ReadDirectoryChangesW` overlapped and waits on the read's event and a stop event with no timeout. The thread owns
//! the handle, the buffer and the `OVERLAPPED`: on the stop event it cancels the read and waits for it to end before
//! they are freed. What a read's bytes say is `super::completion`, safe code.
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE,
    FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_CREATION, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, ResetEvent, SetEvent, WaitForMultipleObjects};
use windows::core::PCWSTR;

use super::{Read, completion};

/// A folder root's tree: names, writes and attributes, never a read (a read changes the last access time only).
pub(super) const FOLDER: FILE_NOTIFY_CHANGE = FILE_NOTIFY_CHANGE(
    FILE_NOTIFY_CHANGE_FILE_NAME.0
        | FILE_NOTIFY_CHANGE_DIR_NAME.0
        | FILE_NOTIFY_CHANGE_ATTRIBUTES.0
        | FILE_NOTIFY_CHANGE_SIZE.0
        | FILE_NOTIFY_CHANGE_LAST_WRITE.0
        | FILE_NOTIFY_CHANGE_CREATION.0,
);
/// A file root's folder: the same, of files.
pub(super) const FILE: FILE_NOTIFY_CHANGE = FILE_NOTIFY_CHANGE(FOLDER.0 & !FILE_NOTIFY_CHANGE_DIR_NAME.0);
/// A folder root's parent: names only, to see the root deleted, renamed or moved away.
pub(super) const PARENT: FILE_NOTIFY_CHANGE =
    FILE_NOTIFY_CHANGE(FILE_NOTIFY_CHANGE_FILE_NAME.0 | FILE_NOTIFY_CHANGE_DIR_NAME.0);

/// libuv's size, which the watch mode spec's Cost counts: a burst bigger than this is an overflow.
const BUFFER: usize = 4096;

/// A handle closed when dropped.
struct Owned(HANDLE);

impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by this module and is closed once, here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

// SAFETY: a kernel handle may be used and closed from any thread; each `Owned` is used by one thread at a time.
unsafe impl Send for Owned {}
// SAFETY: the stop event is only set (`SetEvent`) and waited on, which Windows allows from many threads at once.
unsafe impl Sync for Owned {}

/// A reader of one directory; dropping it stops its thread and returns once the thread has ended.
pub(super) struct Reader {
    stop: std::sync::Arc<Owned>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Reader {
    /// Opens `dir` and reads its changes on a thread of its own until `on` returns false or the reader is dropped.
    /// Returns once the first read is issued: Windows keeps no change made before it.
    pub(super) fn start(
        dir: &Path,
        subtree: bool,
        filter: FILE_NOTIFY_CHANGE,
        on: impl FnMut(Read) -> bool + Send + 'static,
    ) -> io::Result<Reader> {
        let (reader, issued) = Reader::spawn(dir, subtree, filter, on, || {})?;
        let _ = issued.recv(); // closed instead when the read could not be issued
        Ok(reader)
    }

    /// `start` without the wait: `before_read` runs on the thread before its first read (a test holds it there),
    /// and the receiver hears once that read is issued.
    pub(super) fn spawn(
        dir: &Path,
        subtree: bool,
        filter: FILE_NOTIFY_CHANGE,
        mut on: impl FnMut(Read) -> bool + Send + 'static,
        before_read: impl FnOnce() + Send + 'static,
    ) -> io::Result<(Reader, mpsc::Receiver<()>)> {
        let (issued_tx, issued_rx) = mpsc::channel();
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is NUL-terminated and outlives the call; every handle made is owned at once, so an error
        // after it closes it.
        let (handle, event, stop) = unsafe {
            let handle = Owned(CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )?);
            let event = Owned(CreateEventW(None, true, false, PCWSTR::null())?);
            let stop = Owned(CreateEventW(None, true, false, PCWSTR::null())?);
            (handle, event, stop)
        };
        let stop = std::sync::Arc::new(stop);
        let stop_seen = std::sync::Arc::clone(&stop);
        let thread = thread::Builder::new().name("grace-watch-dir".into()).spawn(move || {
            // whole, so the thread owns the handles and closes them when it ends (a closure takes fields apart)
            let (handle, event, stop_seen) = (handle, event, stop_seen);
            before_read();
            // `u32`s, so the entries' fields are aligned as `ReadDirectoryChangesW` requires
            let mut buf = vec![0u32; BUFFER / 4];
            let mut bytes = Vec::with_capacity(BUFFER);
            loop {
                let mut ov = OVERLAPPED {
                    hEvent: event.0,
                    ..Default::default()
                };
                // SAFETY: `buf` and `ov` live on this thread until the read has ended: the loop leaves only after
                // `GetOverlappedResult` waited for it, or when it was never issued.
                let issued = unsafe {
                    let _ = ResetEvent(event.0);
                    ReadDirectoryChangesW(
                        handle.0,
                        buf.as_mut_ptr().cast(),
                        BUFFER as u32,
                        subtree,
                        filter,
                        None,
                        Some(&mut ov),
                        None,
                    )
                };
                if let Err(e) = issued {
                    on(Read::Failed(io::Error::from_raw_os_error(e.code().0 & 0xFFFF)));
                    return;
                }
                let _ = issued_tx.send(());
                // SAFETY: both handles are open for the thread's life.
                let woke = unsafe { WaitForMultipleObjects(&[event.0, stop_seen.0], false, INFINITE) };
                let mut n = 0u32;
                if woke != WAIT_OBJECT_0 {
                    // the stop event, or a wait that failed: cancel the read and wait for it to end before `buf`
                    // and `ov` are freed
                    // SAFETY: `ov` is the read issued above on this handle, and is still alive.
                    unsafe {
                        let _ = CancelIoEx(handle.0, Some(&ov));
                        let _ = GetOverlappedResult(handle.0, &ov, &mut n, true);
                    }
                    return;
                }
                // SAFETY: the read has ended (its event is set); `n` is written by the call.
                let ended = unsafe { GetOverlappedResult(handle.0, &ov, &mut n, false) };
                let result = match ended {
                    Ok(()) => {
                        bytes.clear();
                        bytes.extend(buf.iter().flat_map(|w| w.to_ne_bytes()).take(n as usize));
                        Ok(&bytes[..])
                    }
                    Err(e) => Err((e.code().0 & 0xFFFF) as u32),
                };
                if !on(completion(result)) {
                    return;
                }
            }
        })?;
        Ok((
            Reader {
                stop,
                thread: Some(thread),
            },
            issued_rx,
        ))
    }

    /// Sets the stop event: a read issued after it ends at once, as one waiting does.
    pub(super) fn signal(&self) {
        // SAFETY: the event is open while `self.stop` lives.
        let _ = unsafe { SetEvent(self.stop.0) };
    }

    /// The thread's CPU cycles so far, for the idle test.
    #[cfg(test)]
    pub(super) fn cycles(&self) -> u64 {
        use std::os::windows::io::AsRawHandle;
        cycles(self.thread.as_ref().unwrap().as_raw_handle())
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.signal();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// A thread's CPU cycles so far; a thread that has ended keeps its count.
#[cfg(test)]
pub(super) fn cycles(thread: std::os::windows::io::RawHandle) -> u64 {
    let mut n = 0u64;
    // SAFETY: the handle is a `JoinHandle`'s, open while the caller holds it; `n` outlives the call.
    let _ = unsafe { windows::Win32::System::WindowsProgramming::QueryThreadCycleTime(HANDLE(thread), &mut n) };
    n
}
