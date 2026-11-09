//! What Grace asks of the platform and a test answers instead (migration guide, Architecture, "Seams for tests"):
//! the folder dialog and opening a file with its program. A URL and the clipboard join with their first callers (P3).
//!
//! The app's seams are [`Platform`]. A build with the `e2e` feature reads `GRACE_E2E_SEAMS` and answers from it
//! instead (`crate::e2e::E2eSeams`), as the Playwright tests stubbed `dialog.showOpenDialog`.
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use gpui::AsyncApp;

pub type Answer<T> = Pin<Box<dyn Future<Output = T>>>;

pub trait Seams: 'static {
    /// Asks for a folder, starting in `start` when given (Open…, #82). `None` when the person cancels.
    fn pick_folder(&self, start: Option<PathBuf>, cx: &mut AsyncApp) -> Answer<Option<PathBuf>>;
    /// Opens a file with the program the system gives its type, as Electron's `shell.openPath`: the HTML report in
    /// the default browser (Report).
    fn open_path(&self, path: &std::path::Path, cx: &mut gpui::App);
}

/// The platform's own dialogs.
pub struct Platform;

impl Seams for Platform {
    fn open_path(&self, path: &std::path::Path, cx: &mut gpui::App) {
        cx.open_with_system(path);
    }

    #[cfg(windows)]
    fn pick_folder(&self, start: Option<PathBuf>, _cx: &mut AsyncApp) -> Answer<Option<PathBuf>> {
        windows_dialog::pick_folder(start)
    }

    // GPUI's portal dialog: it cannot start in a given folder, so on Linux, which is not released (guide R5), it
    // opens where the desktop chooses.
    #[cfg(not(windows))]
    fn pick_folder(&self, _start: Option<PathBuf>, cx: &mut AsyncApp) -> Answer<Option<PathBuf>> {
        let picked = cx.update(|cx| {
            cx.prompt_for_paths(gpui::PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: None,
            })
        });
        Box::pin(async move { picked.await.ok()?.ok()?.and_then(|mut paths| paths.pop()) })
    }
}

#[cfg(windows)]
mod windows_dialog {
    //! The Shell's folder dialog with a starting folder, which GPUI's `prompt_for_paths` cannot give (#82, HS-12). It
    //! runs on a thread of its own, owned by Grace's window, so the window stays drawn while it is open.
    use super::Answer;
    use std::path::PathBuf;
    use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
        CoUninitialize,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows::Win32::UI::Shell::{
        FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName,
        SIGDN_FILESYSPATH,
    };
    use windows::core::{HRESULT, HSTRING};

    pub fn pick_folder(start: Option<PathBuf>) -> Answer<Option<PathBuf>> {
        // Called on the UI thread: its active window is Grace's, the dialog's owner.
        let owner = unsafe { GetActiveWindow() }.0 as isize;
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let _ = tx.send_blocking(show(start, HWND(owner as _)));
        });
        Box::pin(async move { rx.recv().await.ok().flatten() })
    }

    fn show(start: Option<PathBuf>, owner: HWND) -> Option<PathBuf> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)
                .ok()
                .ok()?;
            let picked = (|| -> windows::core::Result<Option<PathBuf>> {
                let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
                dialog.SetOptions(dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM)?;
                if let Some(start) = start {
                    let item: windows::core::Result<IShellItem> =
                        SHCreateItemFromParsingName(&HSTRING::from(start.as_os_str()), None);
                    if let Ok(item) = item {
                        dialog.SetFolder(&item)?;
                    }
                }
                let owner = (!owner.0.is_null()).then_some(owner);
                if let Err(e) = dialog.Show(owner) {
                    if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                        return Ok(None);
                    }
                    return Err(e);
                }
                let path = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
                let text = path.to_string();
                windows::Win32::System::Com::CoTaskMemFree(Some(path.0 as _));
                Ok(text.ok().map(PathBuf::from))
            })();
            CoUninitialize();
            picked.ok().flatten()
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::Mutex;

    /// Answers every folder dialog with one folder, and records where each started.
    #[derive(Default)]
    pub struct FakeSeams {
        answer: Option<PathBuf>,
        asked: Mutex<Vec<Option<PathBuf>>>,
        opened: Mutex<Vec<PathBuf>>,
    }

    impl FakeSeams {
        pub fn answering(folder: Option<&str>) -> Self {
            Self {
                answer: folder.map(PathBuf::from),
                asked: Mutex::default(),
                opened: Mutex::default(),
            }
        }
        pub fn asked(&self) -> Vec<Option<PathBuf>> {
            self.asked.lock().unwrap().clone()
        }
        pub fn opened(&self) -> Vec<PathBuf> {
            self.opened.lock().unwrap().clone()
        }
    }

    impl Seams for FakeSeams {
        fn pick_folder(&self, start: Option<PathBuf>, _cx: &mut AsyncApp) -> Answer<Option<PathBuf>> {
            self.asked.lock().unwrap().push(start);
            let answer = self.answer.clone();
            Box::pin(async move { answer })
        }
        fn open_path(&self, path: &std::path::Path, _cx: &mut gpui::App) {
            self.opened.lock().unwrap().push(path.to_path_buf());
        }
    }
}
