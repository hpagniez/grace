//! `src/main/settings.ts`: `settings.json` in the data folder, which holds what Grace remembers between runs: the last
//! and recent project folders, the window, each project's tag filter, environment, run properties, thread count and
//! watch mode, the update settings and the theme. The file is the one Electron Grace writes, key for key.
//!
//! Every call reads the file, changes one key and writes it back, as the TypeScript does: a save never drops the other
//! keys, including those of a newer Grace that this one does not know. Nothing is cached, so the file stays the one
//! source. Loads never fail: a missing, corrupt or unreadable file, or a malformed value, reads as the default. Saves
//! return the I/O error; a save over a file that exists but cannot be read fails and leaves it alone, where the
//! TypeScript also throws, so a transient `EBUSY` never wipes the settings.
//!
//! Writes are atomic as the TypeScript's are: the new file is written beside the old, flushed to the disk, and renamed
//! over it, so a save cut short by a crash or a power loss leaves the old file whole. Beyond the TypeScript:
//! - saves are serialised by a lock, since the read-modify-write that was one turn of Electron's event loop can now race
//!   between threads;
//! - the temporary file is removed when a save fails, and the folder is flushed after the rename (Linux), so the
//!   rename survives a power loss too.
//!
//! The JSON written is JavaScript's: keys in the order `JSON.parse` and `JSON.stringify` give an object (an integer-like
//! key such as an environment named `10` first), whole numbers without `.0`, compact. A value this file does not own is
//! kept as it was read.
//!
//! Differs from the TypeScript, besides the above, in:
//! - Types: a saved row is a `StoredRow` (name, a plain or secret value, `env_var`) and the window a `WindowState` of
//!   `f64`s; the file's keys and their order are the same. Paths are `&str`/`String`, as the model's `Project::root`.
//!   A save that fails gives the `io::Error`, with the system's message where Node gives its own.
//! - `same_folder` ignores case on Windows only: Linux folders `Work` and `work` differ, where the TypeScript, which only
//!   ever ran on Windows, lowercases everywhere. A saved path counts as absolute by Rust's `Path::is_absolute`, which on
//!   Windows also wants a drive or a share (`\foo` is not one; Node's `isAbsolute` says it is).
//! - `lastChecked` is read as an RFC 3339 date, which is what Grace writes (`toISOString()`); the TypeScript takes
//!   whatever `Date.parse` takes, such as `Oct 28 2026`, which nothing here could show.
//! - A number past `f64`'s range (`1e999`) makes `serde_json` reject the whole file as corrupt, where JavaScript reads
//!   `Infinity`: the other keys are then lost at the next save. Nothing writes one.
//! - The tests make a read or a rename fail through thread-local hooks of `cfg(test)` (`hooks`), as the TypeScript's
//!   mock `node:fs`; the library has no seam for it.
use crate::model::env::{Named, name_error, property_value_error, valid_env_name};
use crate::model::js;
use crate::model::tags::{FilterState, parse_filter_state};
use crate::model::theme::{Theme, parse_theme};
use crate::model::threads::{parse_threads, valid_threads};
use crate::model::updates::{Channel, UpdateSettings};
use crate::model::version::parse_version;
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// The whole file: an object of keys this module reads and writes, and keys it keeps.
type Settings = Map<String, Value>;

const NULL: &Value = &Value::Null;

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

// ----- the file -----

/// Makes every read or rename of the test's thread fail, as the TypeScript's tests mock `readFileSync` and
/// `renameSync`: a file another program holds, a rename that is refused.
#[cfg(test)]
mod hooks {
    use std::cell::Cell;
    use std::io::{self, ErrorKind};
    use std::thread::LocalKey;

    type Failure = Cell<Option<ErrorKind>>;

    thread_local! {
        static READ: Failure = const { Cell::new(None) };
        static RENAME: Failure = const { Cell::new(None) };
    }

    pub struct Fails(&'static LocalKey<Failure>);

    impl Drop for Fails {
        fn drop(&mut self) {
            self.0.set(None);
        }
    }

    pub fn reads_fail(kind: ErrorKind) -> Fails {
        READ.set(Some(kind));
        Fails(&READ)
    }

    pub fn renames_fail(kind: ErrorKind) -> Fails {
        RENAME.set(Some(kind));
        Fails(&RENAME)
    }

    pub fn read() -> io::Result<()> {
        READ.get().map_or(Ok(()), |kind| Err(kind.into()))
    }

    pub fn rename() -> io::Result<()> {
        RENAME.get().map_or(Ok(()), |kind| Err(kind.into()))
    }
}

/// The text of the file, as Node's `readFileSync(file, "utf8")` reads it: bytes that are not UTF-8 become U+FFFD.
fn read_text(path: &Path) -> io::Result<String> {
    #[cfg(test)]
    hooks::read()?;
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

/// `value` as JavaScript holds it: the keys of every object in the order it lists them (array indexes first, ascending,
/// then the rest as they came, which `JSON.parse` and an object built key by key both give and `JSON.stringify` writes)
/// and a whole number as an integer.
fn js_order(value: &mut Value) {
    match value {
        Value::Object(map) => js_order_map(map),
        Value::Array(items) => items.iter_mut().for_each(js_order),
        // `1.0` is `1` to JavaScript, which writes it so
        Value::Number(n) if n.is_f64() => {
            if let Some(whole) = n
                .as_f64()
                .filter(|f| f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0)
            {
                *value = Value::from(whole as i64);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn js_order_map(map: &mut Settings) {
    map.values_mut().for_each(js_order);
    let order: Vec<String> = js::key_order(map.keys()).into_iter().cloned().collect();
    if order.iter().eq(map.keys()) {
        return;
    }
    let mut old = std::mem::take(map);
    for key in order {
        if let Some(value) = old.remove(&key) {
            map.insert(key, value);
        }
    }
}

/// The whole settings file. A missing, non-JSON or non-object file reads as empty; any other read error is returned,
/// so a save never overwrites a file it could not read (a transient `EBUSY`, say).
fn read(data_dir: &Path) -> io::Result<Settings> {
    let text = match read_text(&file(data_dir)) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Settings::new()),
        Err(e) => return Err(e),
    };
    Ok(match serde_json::from_str(&text) {
        Ok(Value::Object(mut map)) => {
            js_order_map(&mut map);
            map
        }
        _ => Settings::new(),
    })
}

fn rename(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(test)]
    hooks::rename()?;
    fs::rename(from, to)
}

/// Writes `bytes` to a temporary file beside `path`, flushes it to the disk and renames it over `path`.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let saved = File::create(&tmp)
        .and_then(|mut f| f.write_all(bytes).and_then(|()| f.sync_all()))
        .and_then(|()| rename(&tmp, path));
    if saved.is_err() {
        let _ = fs::remove_file(&tmp);
        return saved;
    }
    // A rename is in the folder, not the file: flush the folder as well (Windows cannot open one to flush it).
    #[cfg(unix)]
    if let Some(dir) = path.parent()
        && let Ok(dir) = File::open(dir)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// One save at a time in this process: two read-modify-writes in turn lose neither change.
static SAVING: Mutex<()> = Mutex::new(());

/// Read-modify-write, so one key never drops the others.
fn update(data_dir: &Path, change: impl FnOnce(&mut Settings)) -> io::Result<()> {
    let _saving = SAVING.lock().unwrap_or_else(PoisonError::into_inner);
    let mut settings = read(data_dir)?;
    change(&mut settings);
    fs::create_dir_all(data_dir)?;
    js_order_map(&mut settings);
    let text = serde_json::to_string(&settings).map_err(io::Error::from)?;
    write_atomic(&file(data_dir), text.as_bytes())
}

/// An object member that is one: itself, or a fresh empty object put in its place.
fn object_mut(value: &mut Value) -> &mut Settings {
    if !value.is_object() {
        *value = Value::Object(Settings::new());
    }
    let Value::Object(map) = value else {
        unreachable!("an object was made above")
    };
    map
}

/// A value of one of this module's types as JSON; `null` if it cannot be (none of them can fail).
fn to_json(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn strings(list: &[String]) -> Value {
    Value::Array(list.iter().cloned().map(Value::String).collect())
}

// ----- the last and the recent projects -----

/// The last opened project folder, or `None` if none is saved, the file is unreadable or the folder is gone.
pub fn load_last_project(data_dir: &Path) -> Option<String> {
    let settings = read(data_dir).ok()?;
    let last = settings.get("lastProject")?.as_str()?;
    Path::new(last).is_dir().then(|| last.to_string())
}

/// How many project folders the recent list keeps.
pub const RECENT_LIMIT: usize = 10;

/// `path.resolve`, lexically: absolute against the working folder, `.` and `..` and the doubled and trailing separators
/// gone.
fn resolved(path: &str) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| PathBuf::from(path));
    let mut parts: Vec<Component> = vec![];
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => match parts.last() {
                Some(Component::Normal(_)) => {
                    parts.pop();
                }
                // there is nothing above a root
                Some(Component::RootDir | Component::Prefix(_)) => {}
                _ => parts.push(part),
            },
            part => parts.push(part),
        }
    }
    parts.iter().collect()
}

/// What two paths of one folder have in common.
fn folder_key(path: &str) -> String {
    let key = resolved(path).to_string_lossy().into_owned();
    // Windows' folders do not tell `D:\Work` from `d:\work`; Linux's do
    if cfg!(windows) { key.to_lowercase() } else { key }
}

/// Two paths are the same folder when they resolve alike, ignoring case on Windows, and a trailing separator.
///
/// The TypeScript ignores case everywhere, as it only ever ran on Windows; a Linux folder `Work` is not `work`.
pub fn same_folder(a: &str, b: &str) -> bool {
    folder_key(a) == folder_key(b)
}

/// The absolute paths of a saved list, newest first, one per folder, at most `RECENT_LIMIT`; anything else is dropped.
fn parse_recent(list: Option<&Value>) -> Vec<String> {
    let mut kept: Vec<String> = vec![];
    let mut keys: Vec<String> = vec![];
    let Some(Value::Array(items)) = list else {
        return kept;
    };
    for item in items {
        if kept.len() == RECENT_LIMIT {
            break;
        }
        let Some(path) = item.as_str().filter(|p| Path::new(p).is_absolute()) else {
            continue;
        };
        let key = folder_key(path);
        if !keys.contains(&key) {
            kept.push(path.to_string());
            keys.push(key);
        }
    }
    kept
}

/// The recent project folders, newest first, as saved (they may no longer exist). Empty if none or the file cannot be
/// read. Never fails.
pub fn load_recent_projects(data_dir: &Path) -> Vec<String> {
    read(data_dir)
        .map(|s| parse_recent(s.get("recentProjects")))
        .unwrap_or_default()
}

/// Saves `root` as the last project and moves it to the top of the recent list.
pub fn save_opened_project(data_dir: &Path, root: &str) -> io::Result<()> {
    update(data_dir, |s| {
        s.insert("lastProject".into(), Value::String(root.to_string()));
        let mut recent = vec![root.to_string()];
        recent.extend(
            parse_recent(s.get("recentProjects"))
                .into_iter()
                .filter(|p| !same_folder(p, root)),
        );
        recent.truncate(RECENT_LIMIT);
        s.insert("recentProjects".into(), strings(&recent));
    })
}

/// Drops `root` from the recent list, and from `lastProject` if that is the same folder.
pub fn remove_recent_project(data_dir: &Path, root: &str) -> io::Result<()> {
    update(data_dir, |s| {
        let recent: Vec<String> = parse_recent(s.get("recentProjects"))
            .into_iter()
            .filter(|p| p != root)
            .collect();
        s.insert("recentProjects".into(), strings(&recent));
        if s.get("lastProject")
            .and_then(Value::as_str)
            .is_some_and(|last| same_folder(last, root))
        {
            s.shift_remove("lastProject");
        }
    })
}

/// Forgets the last project, so the next start shows the home screen.
pub fn forget_last_project(data_dir: &Path) -> io::Result<()> {
    update(data_dir, |s| {
        s.shift_remove("lastProject");
    })
}

// ----- the window -----

/// The window's normal-state bounds and whether it was maximized. Numbers are `f64` as the file's are: it can hold
/// fractions.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct WindowState {
    #[serde(with = "crate::model::js_num")]
    pub x: f64,
    #[serde(with = "crate::model::js_num")]
    pub y: f64,
    #[serde(with = "crate::model::js_num")]
    pub width: f64,
    #[serde(with = "crate::model::js_num")]
    pub height: f64,
    pub maximized: bool,
}

/// The saved window state, or `None` if none is saved, it is malformed or the file cannot be read.
pub fn load_window_state(data_dir: &Path) -> Option<WindowState> {
    let settings = read(data_dir).ok()?;
    let window = settings.get("window")?.as_object()?;
    let number = |key: &str| window.get(key).and_then(js::finite);
    let state = WindowState {
        x: number("x")?,
        y: number("y")?,
        width: number("width")?,
        height: number("height")?,
        maximized: window.get("maximized")?.as_bool()?,
    };
    (state.width > 0.0 && state.height > 0.0).then_some(state)
}

pub fn save_window_state(data_dir: &Path, window: &WindowState) -> io::Result<()> {
    update(data_dir, |s| {
        s.insert("window".into(), to_json(window));
    })
}

// ----- the bottom panel -----

/// The saved bottom panel height (`panelHeight`); the default when none is saved, the value is unusable or the file
/// cannot be read. Clamped when shown, not here.
pub fn load_panel_height(data_dir: &Path) -> f32 {
    read(data_dir)
        .ok()
        .and_then(|s| s.get("panelHeight").and_then(js::finite))
        .filter(|h| *h >= f64::from(crate::panel_height::PANEL_MIN) && *h <= f64::from(f32::MAX))
        .map_or(crate::panel_height::PANEL_DEFAULT, |h| h as f32)
}

/// Saves the height as a whole number. A height that cannot be saved is only forgotten: the caller drops the error.
pub fn save_panel_height(data_dir: &Path, h: f32) -> io::Result<()> {
    update(data_dir, |s| {
        s.insert("panelHeight".into(), Value::from(h.round() as i64));
    })
}

/// The saved order of the bottom panel's tabs (`panelTabs`); the default when none is saved, it is not the three tabs
/// once each, or the file cannot be read.
pub fn load_tab_order(data_dir: &Path) -> [crate::panel_tabs::TabId; 3] {
    crate::panel_tabs::parse_tab_order(read(data_dir).ok().as_ref().and_then(|s| s.get("panelTabs")))
}

/// An order that cannot be saved is only forgotten: the caller drops the error.
pub fn save_tab_order(data_dir: &Path, order: &[crate::panel_tabs::TabId; 3]) -> io::Result<()> {
    update(data_dir, |s| {
        s.insert(
            "panelTabs".into(),
            order.iter().map(|t| t.name()).collect::<Vec<_>>().into(),
        );
    })
}

// ----- a project's entry -----

/// A saved property: a plain value, or a secret as base64 ciphertext.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoredValue {
    Plain(String),
    Secret(String),
}

/// A saved run property. A row is a system property, or an environment variable (`env_var`).
///
/// The TypeScript's `StoredRow` is a union of `{ name, value }` and `{ name, secret }` with an optional `envVar: true`;
/// here the three are fields, and the file has the same keys in the same order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRow {
    pub name: String,
    pub value: StoredValue,
    pub env_var: bool,
}

impl StoredRow {
    /// A system property with a plain value.
    pub fn plain(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: StoredValue::Plain(value.into()),
            env_var: false,
        }
    }

    /// A system property whose value is a secret, as the ciphertext.
    pub fn secret(name: impl Into<String>, ciphertext: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: StoredValue::Secret(ciphertext.into()),
            env_var: false,
        }
    }

    /// The same row, as an environment variable.
    pub fn as_env_var(mut self) -> Self {
        self.env_var = true;
        self
    }

    /// As the file has it: `name`, then `value` or `secret`, then `envVar` when it is one.
    fn to_json(&self) -> Value {
        let mut row = Map::new();
        row.insert("name".into(), Value::String(self.name.clone()));
        match &self.value {
            StoredValue::Plain(v) => row.insert("value".into(), Value::String(v.clone())),
            StoredValue::Secret(s) => row.insert("secret".into(), Value::String(s.clone())),
        };
        if self.env_var {
            row.insert("envVar".into(), Value::Bool(true));
        }
        Value::Object(row)
    }
}

/// A saved row has a name, so the runner takes a run's rows with `model::env::effective_rows`, as `runner.ts` does.
impl Named for StoredRow {
    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoredLists {
    pub shared: Vec<StoredRow>,
    pub by_env: IndexMap<String, Vec<StoredRow>>,
}

/// A project's environment and run properties. The TypeScript's `emptyEnvSettings()` is `default()`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvSettings {
    pub env: Option<String>,
    pub properties: StoredLists,
}

/// Changes one project's entry and keeps its other fields; an entry that is not an object starts fresh.
fn update_entry(data_dir: &Path, root: &str, change: impl FnOnce(&mut Settings)) -> io::Result<()> {
    update(data_dir, |s| {
        let projects = object_mut(s.entry("projects").or_insert(Value::Null));
        change(object_mut(projects.entry(root).or_insert(Value::Null)));
    })
}

/// The saved entry of a project, as written; `None` if none. Fails if the file cannot be read.
fn saved_entry(data_dir: &Path, root: &str) -> io::Result<Option<Value>> {
    Ok(match read(data_dir)?.remove("projects") {
        Some(Value::Object(mut projects)) => projects.remove(root),
        _ => None,
    })
}

/// The saved tag filter of a project, or no filter if none is saved, the entry is malformed or the file cannot be
/// read. Never fails.
pub fn load_project_filter(data_dir: &Path, root: &str) -> FilterState {
    saved_entry(data_dir, root)
        .ok()
        .flatten()
        .and_then(|entry| parse_filter_state(&entry))
        .unwrap_or_default()
}

pub fn save_project_filter(data_dir: &Path, root: &str, f: &FilterState) -> io::Result<()> {
    update_entry(data_dir, root, |e| {
        e.insert("include".into(), strings(&f.include));
        e.insert("exclude".into(), strings(&f.exclude));
        e.insert("hideNonMatching".into(), Value::Bool(f.hide_non_matching));
    })
}

/// A row needs a name its kind allows and exactly one of a plain value that Java can take or a non-empty secret. Its
/// kind is `envVar: true` or absent (a system property).
fn parse_stored_row(x: &Value) -> Option<StoredRow> {
    let row = x.as_object()?;
    let env_var = match row.get("envVar") {
        None => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return None,
    };
    let name = row.get("name")?.as_str()?;
    if name_error(name, env_var).is_some() {
        return None;
    }
    let value = match (row.get("value"), row.get("secret")) {
        (Some(Value::String(v)), None) if property_value_error(v).is_none() => StoredValue::Plain(v.clone()),
        (None, Some(Value::String(s))) if !s.is_empty() => StoredValue::Secret(s.clone()),
        _ => return None,
    };
    Some(StoredRow {
        name: name.to_string(),
        value,
        env_var,
    })
}

/// The rows of a saved list that parse; a hole or anything that is not a row is dropped.
fn parse_stored_rows(list: Option<&Value>) -> Vec<StoredRow> {
    match list {
        Some(Value::Array(rows)) => rows.iter().filter_map(parse_stored_row).collect(),
        _ => vec![],
    }
}

fn parse_env_settings(entry: Option<&Value>) -> EnvSettings {
    let mut out = EnvSettings::default();
    let Some(entry) = entry else { return out };
    if let Some(env) = entry.get("env").and_then(Value::as_str).filter(|e| valid_env_name(e)) {
        out.env = Some(env.to_string());
    }
    let Some(properties) = entry.get("properties").filter(|p| p.is_object()) else {
        return out;
    };
    out.properties.shared = parse_stored_rows(properties.get("shared"));
    if let Some(Value::Object(by_env)) = properties.get("byEnv") {
        for (name, rows) in by_env {
            if valid_env_name(name) && rows.is_array() {
                out.properties
                    .by_env
                    .insert(name.clone(), parse_stored_rows(Some(rows)));
            }
        }
    }
    out
}

/// The saved environment and run properties of a project. Malformed parts read as missing and an unreadable file as no
/// environment, so it never fails.
pub fn load_project_env(data_dir: &Path, root: &str) -> EnvSettings {
    match saved_entry(data_dir, root) {
        Ok(entry) => parse_env_settings(entry.as_ref()),
        Err(_) => EnvSettings::default(),
    }
}

/// The saved thread count of a project; 1 if none is saved, it is malformed or the file cannot be read. Never fails.
pub fn load_project_threads(data_dir: &Path, root: &str) -> u32 {
    saved_entry(data_dir, root)
        .ok()
        .flatten()
        .and_then(|entry| parse_threads(entry.get("threads").unwrap_or(NULL)))
        .unwrap_or(1)
}

pub fn save_project_threads(data_dir: &Path, root: &str, n: u32) -> io::Result<()> {
    update_entry(data_dir, root, |e| {
        e.insert("threads".into(), Value::from(n));
    })
}

/// Whether watch mode is saved on for a project; off if not saved, malformed or unreadable. Never fails.
pub fn load_project_watch(data_dir: &Path, root: &str) -> bool {
    saved_entry(data_dir, root)
        .ok()
        .flatten()
        .is_some_and(|entry| entry.get("watch") == Some(&Value::Bool(true)))
}

/// On is saved as `watch: true`; off removes the key.
pub fn save_project_watch(data_dir: &Path, root: &str, on: bool) -> io::Result<()> {
    update_entry(data_dir, root, |e| {
        if on {
            e.insert("watch".into(), Value::Bool(true));
        } else {
            e.shift_remove("watch");
        }
    })
}

/// The rows of a saved list that the rules of today reject, as written.
fn rejected_rows(list: Option<&Value>) -> Vec<Value> {
    match list {
        Some(Value::Array(rows)) => rows.iter().filter(|r| parse_stored_row(r).is_none()).cloned().collect(),
        _ => vec![],
    }
}

/// `<scope>: <name>` of each rejected row in a list that loads (`(no name)` if it has none), never its value: it may be
/// a secret. Never fails.
pub fn skipped_properties(data_dir: &Path, root: &str) -> Vec<String> {
    let Ok(entry) = saved_entry(data_dir, root) else {
        return vec![];
    };
    let properties = entry.as_ref().and_then(|e| e.get("properties"));
    let named = |scope: &str, rows: Option<&Value>| -> Vec<String> {
        rejected_rows(rows)
            .iter()
            .map(|r| {
                let name = r.get("name").and_then(Value::as_str).filter(|n| !n.is_empty());
                format!("{scope}: {}", name.unwrap_or("(no name)"))
            })
            .collect()
    };
    let mut skipped = named("All environments", properties.and_then(|p| p.get("shared")));
    if let Some(Value::Object(by_env)) = properties.and_then(|p| p.get("byEnv")) {
        for (env, rows) in by_env {
            if valid_env_name(env) {
                skipped.extend(named(env, Some(rows)));
            }
        }
    }
    skipped
}

/// The rows of the file that the rules of today reject stay after the new rows of their list while that list exists, so
/// a newer rule never erases them. Of the old `properties` object nothing else is kept.
fn put_properties(e: &mut Settings, p: &StoredLists) {
    let old = e.get("properties");
    let old_by_env = old.and_then(|o| o.get("byEnv")).and_then(Value::as_object);
    let list = |rows: &[StoredRow], rejected: Vec<Value>| -> Value {
        Value::Array(rows.iter().map(StoredRow::to_json).chain(rejected).collect())
    };
    let mut by_env = Settings::new();
    for (env, rows) in &p.by_env {
        let rejected = rejected_rows(old_by_env.and_then(|m| m.get(env)));
        by_env.insert(env.clone(), list(rows, rejected));
    }
    let mut properties = Settings::new();
    properties.insert(
        "shared".into(),
        list(&p.shared, rejected_rows(old.and_then(|o| o.get("shared")))),
    );
    properties.insert("byEnv".into(), Value::Object(by_env));
    e.insert("properties".into(), Value::Object(properties));
}

/// Saves the environment, `(none)` as null (docs/specs/2026-11-04-pom-defaults.md R1.3), and the lists.
pub fn save_project_env(data_dir: &Path, root: &str, s: &EnvSettings) -> io::Result<()> {
    update_entry(data_dir, root, |e| {
        e.insert("env".into(), s.env.clone().map_or(Value::Null, Value::String));
        put_properties(e, &s.properties);
    })
}

/// Saves the lists only: the environment stays as saved, or unsaved (R1.2).
pub fn save_project_properties(data_dir: &Path, root: &str, p: &StoredLists) -> io::Result<()> {
    update_entry(data_dir, root, |e| put_properties(e, p))
}

/// Which of the environment, the tag filter and the thread count a project has saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedControls {
    pub env: bool,
    pub filter: bool,
    pub threads: bool,
}

/// Which of the environment, the tag filter and the thread count the project has saved (R1.1); `None` when
/// `settings.json` cannot be read, so the caller decides. Never fails.
pub fn saved_controls(data_dir: &Path, root: &str) -> Option<SavedControls> {
    let entry = saved_entry(data_dir, root).ok()?;
    let Some(entry @ Value::Object(fields)) = &entry else {
        return Some(SavedControls {
            env: false,
            filter: false,
            threads: false,
        });
    };
    Some(SavedControls {
        env: match fields.get("env") {
            Some(Value::Null) => true,
            Some(Value::String(env)) => valid_env_name(env),
            _ => false,
        },
        filter: parse_filter_state(entry).is_some(),
        threads: valid_threads(fields.get("threads").unwrap_or(NULL)),
    })
}

// ----- updates and appearance -----

/// Whether `Date.parse` would read the text. Grace writes `toISOString()`, which is RFC 3339, and that is all this
/// reads: `Date.parse` also takes whatever else the engine knows (`Oct 28 2026`), which nothing here writes and a date
/// that is shown could not be parsed from.
fn is_date(text: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(text).is_ok()
}

/// Update settings from any JSON value (R12). Each malformed or missing field reads as its default, the channel as not
/// picked (R1.3). A `skipped` or `failed` that is not a valid version reads as `None`. Never fails.
pub fn parse_update_settings(updates: &Value) -> UpdateSettings {
    let d = UpdateSettings::default();
    let boolean = |key: &str, default: bool| updates.get(key).and_then(Value::as_bool).unwrap_or(default);
    let version = |key: &str| updates.get(key).and_then(Value::as_str).and_then(parse_version);
    UpdateSettings {
        channel: updates
            .get("channel")
            .and_then(|c| serde_json::from_value::<Channel>(c.clone()).ok())
            .or(d.channel),
        check_at_startup: boolean("checkAtStartup", d.check_at_startup),
        auto_update: boolean("autoUpdate", d.auto_update),
        skipped: version("skipped"),
        failed: version("failed"),
        last_checked: updates
            .get("lastChecked")
            .and_then(Value::as_str)
            .filter(|d| is_date(d))
            .map(String::from),
    }
}

/// The saved update settings (R12). An unreadable file reads as the defaults, so it never fails.
pub fn load_update_settings(data_dir: &Path) -> UpdateSettings {
    match read(data_dir) {
        Ok(settings) => parse_update_settings(settings.get("updates").unwrap_or(NULL)),
        Err(_) => UpdateSettings::default(),
    }
}

/// Saves the update settings and keeps the other keys. A channel not picked is left out, so the default keeps following
/// the running version (R1.3).
pub fn save_update_settings(data_dir: &Path, s: &UpdateSettings) -> io::Result<()> {
    update(data_dir, |all| {
        let mut updates = Settings::new();
        if let Some(channel) = &s.channel {
            updates.insert("channel".into(), to_json(channel));
        }
        updates.insert("checkAtStartup".into(), Value::Bool(s.check_at_startup));
        updates.insert("autoUpdate".into(), Value::Bool(s.auto_update));
        updates.insert("skipped".into(), to_json(&s.skipped));
        updates.insert("failed".into(), to_json(&s.failed));
        updates.insert(
            "lastChecked".into(),
            s.last_checked.clone().map_or(Value::Null, Value::String),
        );
        all.insert("updates".into(), Value::Object(updates));
    })
}

/// The saved theme (spec R8.2). A missing or malformed key or value, or an unreadable file, reads as Follow Windows, so
/// it never fails.
pub fn load_appearance(data_dir: &Path) -> Theme {
    match read(data_dir) {
        Ok(settings) => {
            let theme = settings.get("appearance").and_then(|a| a.get("theme"));
            parse_theme(theme.unwrap_or(NULL))
        }
        Err(_) => Theme::System,
    }
}

/// Saves the theme and keeps the other keys.
pub fn save_appearance(data_dir: &Path, theme: Theme) -> io::Result<()> {
    update(data_dir, |s| {
        let mut appearance = Settings::new();
        appearance.insert("theme".into(), Value::String(theme.as_str().to_string()));
        s.insert("appearance".into(), Value::Object(appearance));
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tags::no_filter;
    use crate::model::version::Version;
    use serde_json::json;
    use std::io::ErrorKind;
    use std::thread;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::Builder::new().prefix("kk-settings-").tempdir().unwrap()
    }

    /// A folder `name` in a temporary folder, and its path.
    fn folder(parent: &TempDir, name: &str) -> String {
        let dir = parent.path().join(name);
        fs::create_dir(&dir).unwrap();
        dir.to_string_lossy().into_owned()
    }

    /// An absolute path that does not exist: `D:\name` on Windows, where the TypeScript's tests use it, and `/d/name`.
    fn abs(name: &str) -> String {
        if cfg!(windows) {
            format!("D:\\{name}")
        } else {
            format!("/d/{name}")
        }
    }

    fn settings_file(data: &TempDir) -> PathBuf {
        data.path().join("settings.json")
    }

    fn write_text(data: &TempDir, text: &str) {
        fs::write(settings_file(data), text).unwrap();
    }

    fn write_settings(data: &TempDir, settings: &Value) {
        write_text(data, &settings.to_string());
    }

    fn read_text_of(data: &TempDir) -> String {
        fs::read_to_string(settings_file(data)).unwrap()
    }

    fn read_json(data: &TempDir) -> Value {
        serde_json::from_str(&read_text_of(data)).unwrap()
    }

    fn strs(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn save_then_load_returns_the_folder() {
        let (data, parent) = (tmp(), tmp());
        let project = folder(&parent, "proj");
        save_opened_project(data.path(), &project).unwrap();
        assert_eq!(load_last_project(data.path()), Some(project));
    }

    #[test]
    fn load_last_project_ignores_a_missing_folder() {
        let (data, parent) = (tmp(), tmp());
        let project = folder(&parent, "gone");
        save_opened_project(data.path(), &project).unwrap();
        fs::remove_dir(&project).unwrap();
        assert_eq!(load_last_project(data.path()), None);
    }

    #[test]
    fn returns_null_when_there_is_no_settings_file() {
        assert_eq!(load_last_project(tmp().path()), None);
    }

    #[test]
    fn returns_null_for_a_corrupt_settings_json() {
        let data = tmp();
        write_text(&data, "{not json");
        assert_eq!(load_last_project(data.path()), None);
    }

    mod recent_projects {
        use super::*;

        #[test]
        fn save_opened_project_puts_the_folder_first_and_saves_last_project() {
            let (data, parent) = (tmp(), tmp());
            let (a, b) = (folder(&parent, "a"), folder(&parent, "b"));
            save_opened_project(data.path(), &a).unwrap();
            save_opened_project(data.path(), &b).unwrap();
            save_opened_project(data.path(), &a).unwrap();
            assert_eq!(load_recent_projects(data.path()), [a.clone(), b]);
            assert_eq!(load_last_project(data.path()), Some(a));
        }

        #[test]
        fn keeps_the_10_newest_folders() {
            let data = tmp();
            for i in 0..=10 {
                save_opened_project(data.path(), &abs(&format!("p{i}"))).unwrap();
            }
            assert_eq!(RECENT_LIMIT, 10);
            let newest: Vec<String> = (0..10).map(|i| abs(&format!("p{}", 10 - i))).collect();
            assert_eq!(load_recent_projects(data.path()), newest);
        }

        #[test]
        fn same_folder_with_a_trailing_separator_or_another_case_is_one_entry() {
            let data = tmp();
            // Windows ignores case in a path; Linux does not, and has only the separator
            let (first, again, last) = if cfg!(windows) {
                ("D:\\work\\api", "d:\\WORK\\API\\", "d:\\WORK\\api")
            } else {
                ("/d/work/api", "/d/work/api/", "/d/work/api/")
            };
            write_settings(&data, &json!({ "recentProjects": [first, again] }));
            assert_eq!(load_recent_projects(data.path()), [first]);
            save_opened_project(data.path(), last).unwrap();
            assert_eq!(load_recent_projects(data.path()), [last]);
        }

        #[test]
        fn a_broken_list_reads_as_what_can_be_kept() {
            let data = tmp();
            let v: Vec<String> = (0..12).map(|i| abs(&format!("v{i}"))).collect();
            let a = abs("a");
            let again = if cfg!(windows) {
                "d:\\A".to_string()
            } else {
                format!("{a}/")
            };
            let mut list = vec![json!(42), json!("rel\\path"), json!(a), json!(again)];
            list.extend(v.iter().map(|p| json!(p)));
            write_settings(&data, &json!({ "recentProjects": list }));
            let mut kept = vec![a.clone()];
            kept.extend(v[..9].iter().cloned());
            assert_eq!(load_recent_projects(data.path()), kept);
            write_settings(&data, &json!({ "recentProjects": "x" }));
            assert!(load_recent_projects(data.path()).is_empty());
            write_settings(&data, &json!({ "lastProject": a }));
            assert!(load_recent_projects(data.path()).is_empty());
            write_text(&data, "{not json");
            assert!(load_recent_projects(data.path()).is_empty());
            assert!(load_recent_projects(tmp().path()).is_empty());
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert!(load_recent_projects(data.path()).is_empty());
        }

        #[test]
        fn remove_recent_project_drops_the_row_and_a_matching_last_project() {
            let data = tmp();
            let (gone, b) = (abs("gone"), abs("b"));
            write_settings(
                &data,
                &json!({
                    "lastProject": gone,
                    "recentProjects": [gone, b],
                    "projects": { (gone.as_str()): { "threads": 2 } },
                }),
            );
            remove_recent_project(data.path(), &gone).unwrap();
            let saved = read_json(&data);
            assert!(saved.get("lastProject").is_none());
            assert_eq!(saved["recentProjects"], json!([b]));
            assert_eq!(saved["projects"], json!({ (gone.as_str()): { "threads": 2 } }));
        }

        #[test]
        fn remove_recent_project_keeps_a_last_project_that_is_another_folder() {
            let data = tmp();
            let (gone, b) = (abs("gone"), abs("b"));
            write_settings(&data, &json!({ "lastProject": b, "recentProjects": [gone, b] }));
            remove_recent_project(data.path(), &gone).unwrap();
            assert_eq!(read_json(&data), json!({ "lastProject": b, "recentProjects": [b] }));
        }

        #[test]
        fn forget_last_project_keeps_the_other_keys() {
            let data = tmp();
            let a = abs("a");
            let saved = json!({
                "recentProjects": [a],
                "window": { "x": 1, "y": 2, "width": 700, "height": 500, "maximized": false },
                "projects": { (a.as_str()): { "threads": 2 } },
            });
            let mut with_last = saved.clone();
            with_last["lastProject"] = json!(a);
            write_settings(&data, &with_last);
            forget_last_project(data.path()).unwrap();
            assert_eq!(read_json(&data), saved);
        }

        // Not in the TypeScript: what `same_folder` makes of a path is the platform's.
        #[test]
        fn same_folder_resolves_dots_and_separators() {
            let (long, short) = if cfg!(windows) {
                ("D:\\a\\..\\b\\.\\c\\", "D:/b/c")
            } else {
                ("/d/a/../b/./c//", "/d/b/c")
            };
            assert!(same_folder(long, short));
            assert!(!same_folder(short, &format!("{short}x")));
            assert!(same_folder(&abs("x"), &abs("x")));
        }

        #[cfg(not(windows))]
        #[test]
        fn same_folder_tells_cases_apart_on_linux() {
            assert!(!same_folder("/d/Work", "/d/work"));
        }

        #[cfg(windows)]
        #[test]
        fn same_folder_ignores_case_on_windows() {
            assert!(same_folder("D:\\Work\\Api", "d:\\work\\API\\"));
        }
    }

    mod tag_filter {
        use super::*;

        fn a() -> FilterState {
            FilterState {
                include: strs(&["smoke"]),
                exclude: strs(&["wip"]),
                hide_non_matching: true,
            }
        }

        fn b() -> FilterState {
            FilterState {
                include: vec![],
                exclude: strs(&["slow"]),
                hide_non_matching: false,
            }
        }

        #[test]
        fn filter_round_trips_per_project() {
            let data = tmp();
            save_project_filter(data.path(), "proj-one", &a()).unwrap();
            save_project_filter(data.path(), "proj-two", &b()).unwrap();
            assert_eq!(load_project_filter(data.path(), "proj-one"), a());
            assert_eq!(load_project_filter(data.path(), "proj-two"), b());
            assert_eq!(load_project_filter(data.path(), "proj-three"), no_filter());
        }

        #[test]
        fn save_opened_project_keeps_saved_filters() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            save_project_filter(data.path(), &project, &a()).unwrap();
            save_opened_project(data.path(), &project).unwrap();
            assert_eq!(load_project_filter(data.path(), &project), a());
            assert_eq!(load_last_project(data.path()), Some(project));
        }

        #[test]
        fn save_project_filter_keeps_last_project() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            save_opened_project(data.path(), &project).unwrap();
            save_project_filter(data.path(), &project, &a()).unwrap();
            assert_eq!(load_last_project(data.path()), Some(project));
        }

        #[test]
        fn a_malformed_entry_reads_as_no_filter() {
            let data = tmp();
            write_settings(&data, &json!({ "projects": { "x": { "include": "smoke" } } }));
            assert_eq!(load_project_filter(data.path(), "x"), no_filter());
            write_text(&data, "{not json");
            assert_eq!(load_project_filter(data.path(), "x"), no_filter());
        }

        #[test]
        fn saving_over_a_corrupt_file_starts_fresh() {
            let data = tmp();
            write_text(&data, "{not json");
            save_project_filter(data.path(), "x", &a()).unwrap();
            assert_eq!(load_project_filter(data.path(), "x"), a());
        }

        #[test]
        fn a_file_that_cannot_be_read_fails_the_save_and_stays_as_it_was() {
            let data = tmp();
            let before = json!({ "lastProject": "p", "projects": { "x": a() } }).to_string();
            write_text(&data, &before);
            {
                let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
                let err = save_project_filter(data.path(), "y", &b()).unwrap_err();
                assert_eq!(err.kind(), ErrorKind::ResourceBusy);
                let err = save_opened_project(data.path(), "q").unwrap_err();
                assert_eq!(err.kind(), ErrorKind::ResourceBusy);
                assert_eq!(load_project_filter(data.path(), "x"), no_filter());
                assert_eq!(load_last_project(data.path()), None);
            }
            assert_eq!(read_text_of(&data), before);
        }

        /// In the TypeScript the no-filter is a shared constant that must be copied; here every call has its own.
        #[test]
        fn the_returned_no_filter_is_a_copy() {
            let mut f = load_project_filter(tmp().path(), "x");
            f.include.push("smoke".to_string());
            assert!(no_filter().include.is_empty());
            assert!(load_project_filter(tmp().path(), "x").include.is_empty());
        }
    }

    mod environment_and_run_properties {
        use super::*;

        fn filter() -> FilterState {
            FilterState {
                include: strs(&["smoke"]),
                exclude: strs(&["wip"]),
                hide_non_matching: true,
            }
        }

        fn env() -> EnvSettings {
            EnvSettings {
                env: Some("qa".into()),
                properties: StoredLists {
                    shared: vec![StoredRow::plain("timeout", "5000")],
                    by_env: IndexMap::from([(
                        "qa".to_string(),
                        vec![
                            StoredRow::plain("timeout", "9000"),
                            StoredRow::secret("api.token", "djEwYmFzZTY0"),
                        ],
                    )]),
                },
            }
        }

        fn write_entry(data: &TempDir, entry: Value) {
            write_settings(data, &json!({ "projects": { "x": entry } }));
        }

        fn entry_of(data: &TempDir) -> Value {
            read_json(data)["projects"]["x"].clone()
        }

        #[test]
        fn env_settings_round_trip() {
            let data = tmp();
            save_project_env(data.path(), "proj-one", &env()).unwrap();
            assert_eq!(load_project_env(data.path(), "proj-one"), env());
            assert_eq!(load_project_env(data.path(), "proj-two"), EnvSettings::default());
        }

        #[test]
        fn saving_the_filter_keeps_env_and_properties() {
            let data = tmp();
            save_project_env(data.path(), "x", &env()).unwrap();
            save_project_filter(data.path(), "x", &filter()).unwrap();
            assert_eq!(load_project_env(data.path(), "x"), env());
            assert_eq!(load_project_filter(data.path(), "x"), filter());
        }

        #[test]
        fn saving_env_keeps_the_filter() {
            let data = tmp();
            save_project_filter(data.path(), "x", &filter()).unwrap();
            save_project_env(data.path(), "x", &env()).unwrap();
            assert_eq!(load_project_filter(data.path(), "x"), filter());
            assert_eq!(load_project_env(data.path(), "x"), env());
        }

        #[test]
        fn an_entry_from_the_tag_filter_version_loads_as_no_environment() {
            let data = tmp();
            write_entry(&data, json!(filter()));
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
        }

        #[test]
        fn malformed_rows_are_dropped_good_rows_kept() {
            let data = tmp();
            let good = json!({ "name": "good", "value": "1" });
            write_entry(
                &data,
                json!({
                    "env": "qa",
                    "properties": {
                        "shared": [
                            good,
                            { "name": "both", "value": "1", "secret": "abc" },
                            { "name": "karate.env", "value": "dev" },
                            { "name": "num", "value": 5 },
                            { "name": "empty", "secret": "" },
                            { "name": "multi", "value": "a\nb" },
                            { "name": "wide", "value": "café" },
                            { "value": "no name" },
                            "text",
                            null,
                            { "name": "secret", "secret": "abc" },
                        ],
                        "byEnv": {
                            "qa": [good],
                            "dev": "not an array",
                            "bad name!": [good],
                            "nothing": [{ "name": 1 }],
                        },
                    },
                }),
            );
            let good = StoredRow::plain("good", "1");
            assert_eq!(
                load_project_env(data.path(), "x"),
                EnvSettings {
                    env: Some("qa".into()),
                    properties: StoredLists {
                        shared: vec![good.clone(), StoredRow::secret("secret", "abc")],
                        by_env: IndexMap::from([("qa".to_string(), vec![good]), ("nothing".to_string(), vec![])]),
                    },
                }
            );
            // only the rows of lists that load are named
            assert_eq!(
                skipped_properties(data.path(), "x"),
                [
                    "All environments: both",
                    "All environments: karate.env",
                    "All environments: num",
                    "All environments: empty",
                    "All environments: multi",
                    "All environments: wide",
                    "All environments: (no name)",
                    "All environments: (no name)",
                    "All environments: (no name)",
                    "nothing: (no name)",
                ]
            );
        }

        #[test]
        fn rejected_rows_are_named_and_a_save_keeps_them_while_their_list_exists() {
            let data = tmp();
            let good = json!({ "name": "good", "value": "1" });
            let reserved = json!({ "name": "java.home", "value": "C:/jdk" });
            let wide = json!({ "name": "greeting", "secret": "abc", "value": "h\u{e9}llo" });
            write_entry(
                &data,
                json!({
                    "include": ["smoke"],
                    "properties": {
                        "shared": [good, reserved],
                        "byEnv": { "qa": [wide, good], "dev": [{ "value": "x" }] },
                    },
                }),
            );
            let good_row = StoredRow::plain("good", "1");
            assert_eq!(
                load_project_env(data.path(), "x").properties,
                StoredLists {
                    shared: vec![good_row.clone()],
                    by_env: IndexMap::from([("qa".to_string(), vec![good_row.clone()]), ("dev".to_string(), vec![])]),
                }
            );
            assert_eq!(
                skipped_properties(data.path(), "x"),
                ["All environments: java.home", "qa: greeting", "dev: (no name)"]
            );
            // after the new rows of each list still there; the rows of the deleted dev list go with it
            let fresh = StoredRow::plain("fresh", "2");
            save_project_env(
                data.path(),
                "x",
                &EnvSettings {
                    env: Some("qa".into()),
                    properties: StoredLists {
                        shared: vec![fresh.clone()],
                        by_env: IndexMap::from([("qa".to_string(), vec![good_row]), ("prod".to_string(), vec![fresh])]),
                    },
                },
            )
            .unwrap();
            let fresh = json!({ "name": "fresh", "value": "2" });
            assert_eq!(
                entry_of(&data),
                json!({
                    "include": ["smoke"],
                    "env": "qa",
                    "properties": {
                        "shared": [fresh, reserved],
                        "byEnv": { "qa": [good, wide], "prod": [fresh] },
                    },
                })
            );
            assert_eq!(
                skipped_properties(data.path(), "x"),
                ["All environments: java.home", "qa: greeting"]
            );
            // a save never copies a valid row of the file
            save_project_env(data.path(), "x", &EnvSettings::default()).unwrap();
            assert_eq!(
                entry_of(&data)["properties"],
                json!({ "shared": [reserved], "byEnv": {} })
            );
        }

        #[test]
        fn an_environment_variable_row_keeps_its_kind_a_row_without_one_is_a_system_property() {
            let data = tmp();
            let variable = json!({ "name": "APP_X", "secret": "c2VjcmV0", "envVar": true });
            let plain_var = json!({ "name": "LOG_LEVEL", "value": "debug", "envVar": true });
            let old = json!({ "name": "timeout", "value": "5000" });
            write_entry(
                &data,
                json!({ "properties": { "shared": [old, variable, plain_var], "byEnv": {} } }),
            );
            let shared = vec![
                StoredRow::plain("timeout", "5000"),
                StoredRow::secret("APP_X", "c2VjcmV0").as_env_var(),
                StoredRow::plain("LOG_LEVEL", "debug").as_env_var(),
            ];
            assert_eq!(load_project_env(data.path(), "x").properties.shared, shared);
            save_project_env(data.path(), "x", &load_project_env(data.path(), "x")).unwrap();
            assert_eq!(load_project_env(data.path(), "x").properties.shared, shared);
        }

        #[test]
        fn a_malformed_kind_or_a_name_its_kind_refuses_is_named_and_kept_not_loaded() {
            let data = tmp();
            let malformed = json!({ "name": "A", "value": "1", "envVar": "yes" });
            let dotted = json!({ "name": "db.user", "value": "1", "envVar": true });
            let off = json!({ "name": "B", "value": "1", "envVar": false });
            write_entry(
                &data,
                json!({ "properties": { "shared": [malformed, dotted, off], "byEnv": {} } }),
            );
            assert!(load_project_env(data.path(), "x").properties.shared.is_empty());
            assert_eq!(
                skipped_properties(data.path(), "x"),
                [
                    "All environments: A",
                    "All environments: db.user",
                    "All environments: B"
                ]
            );
            save_project_env(data.path(), "x", &load_project_env(data.path(), "x")).unwrap();
            assert_eq!(entry_of(&data)["properties"]["shared"], json!([malformed, dotted, off]));
        }

        #[test]
        fn skipped_properties_never_throws() {
            let data = tmp();
            assert!(skipped_properties(data.path(), "x").is_empty());
            write_entry(
                &data,
                json!({ "properties": { "shared": [{ "name": "java.home", "value": "x" }] } }),
            );
            {
                let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
                assert!(skipped_properties(data.path(), "x").is_empty());
            }
            write_text(&data, "{not json");
            assert!(skipped_properties(data.path(), "x").is_empty());
        }

        #[test]
        fn a_bad_env_bad_lists_and_unsafe_keys_read_as_missing() {
            let data = tmp();
            write_entry(
                &data,
                json!({ "env": "not valid!", "properties": { "shared": "x", "byEnv": [] } }),
            );
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
            write_entry(&data, json!({ "env": 5, "properties": "x" }));
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
            write_entry(&data, json!("text"));
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
            write_text(
                &data,
                r#"{"projects":{"x":{"properties":{"byEnv":{"__proto__":[{"name":"a","value":"1"}]}}}}}"#,
            );
            // an environment must start with a letter or a digit: `__proto__` is no name, and is dropped
            assert!(load_project_env(data.path(), "x").properties.by_env.is_empty());
            write_text(&data, "{not json");
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
        }

        #[test]
        fn a_sparse_row_list_loses_only_its_holes() {
            let data = tmp();
            write_text(
                &data,
                r#"{"projects":{"x":{"properties":{"shared":[{"name":"a","value":"1"},null,{"name":"b","value":"2"}]}}}}"#,
            );
            let names: Vec<String> = load_project_env(data.path(), "x")
                .properties
                .shared
                .into_iter()
                .map(|r| r.name)
                .collect();
            assert_eq!(names, ["a", "b"]);
        }

        #[test]
        fn env_null_is_saved_as_null() {
            let data = tmp();
            save_project_env(data.path(), "x", &env()).unwrap();
            save_project_env(data.path(), "x", &EnvSettings { env: None, ..env() }).unwrap();
            let entry = entry_of(&data);
            assert_eq!(entry["env"], Value::Null);
            assert_eq!(entry["properties"]["byEnv"]["qa"][1]["secret"], "djEwYmFzZTY0");
            assert_eq!(load_project_env(data.path(), "x").env, None);
        }

        #[test]
        fn load_project_env_does_not_throw_when_the_file_cannot_be_read() {
            let data = tmp();
            save_project_env(data.path(), "x", &env()).unwrap();
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_project_env(data.path(), "x"), EnvSettings::default());
            let err = save_project_env(data.path(), "x", &env()).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::ResourceBusy);
        }

        #[test]
        fn save_project_properties_writes_the_lists_and_keeps_the_environment() {
            let data = tmp();
            save_project_env(data.path(), "x", &env()).unwrap();
            let properties = StoredLists {
                shared: vec![StoredRow::plain("a", "1")],
                by_env: IndexMap::new(),
            };
            save_project_properties(data.path(), "x", &properties).unwrap();
            assert_eq!(
                load_project_env(data.path(), "x"),
                EnvSettings {
                    env: Some("qa".into()),
                    properties
                }
            );
        }

        #[test]
        fn save_project_properties_on_a_fresh_entry_writes_no_environment() {
            let data = tmp();
            save_project_properties(data.path(), "x", &env().properties).unwrap();
            assert!(entry_of(&data).get("env").is_none());
            assert_eq!(saved_controls(data.path(), "x").map(|c| c.env), Some(false));
        }

        #[test]
        fn the_empty_settings_are_a_fresh_object_each_call() {
            let mut e = EnvSettings::default();
            e.properties.shared.push(StoredRow::plain("a", "1"));
            e.properties.by_env.insert("qa".into(), vec![]);
            assert_eq!(
                EnvSettings::default(),
                EnvSettings {
                    env: None,
                    properties: StoredLists::default()
                }
            );
            assert!(EnvSettings::default().properties.shared.is_empty());
        }
    }

    mod saved_controls {
        use super::*;

        const NONE: SavedControls = SavedControls {
            env: false,
            filter: false,
            threads: false,
        };

        #[test]
        fn none_on_an_empty_data_folder() {
            assert_eq!(saved_controls(tmp().path(), "x"), Some(NONE));
        }

        #[test]
        fn each_control_on_its_own() {
            let data = tmp();
            save_project_threads(data.path(), "x", 2).unwrap();
            assert_eq!(
                saved_controls(data.path(), "x"),
                Some(SavedControls { threads: true, ..NONE })
            );
            save_project_filter(data.path(), "x", &no_filter()).unwrap();
            assert_eq!(
                saved_controls(data.path(), "x"),
                Some(SavedControls {
                    threads: true,
                    filter: true,
                    ..NONE
                })
            );
            save_project_env(data.path(), "x", &EnvSettings::default()).unwrap();
            assert_eq!(
                saved_controls(data.path(), "x"),
                Some(SavedControls {
                    env: true,
                    filter: true,
                    threads: true
                })
            );
        }

        #[test]
        fn an_env_grace_does_not_take_is_not_saved() {
            let data = tmp();
            write_text(&data, r#"{"projects":{"x":{"env":"x y"}}}"#);
            assert_eq!(saved_controls(data.path(), "x").map(|c| c.env), Some(false));
        }

        #[test]
        fn unknown_when_the_file_cannot_be_read() {
            let data = tmp();
            save_project_threads(data.path(), "x", 2).unwrap();
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(saved_controls(data.path(), "x"), None);
        }
    }

    mod watch_mode {
        use super::*;

        #[test]
        fn is_off_with_nothing_saved_on_once_saved_on_and_off_saved_removes_the_key() {
            let data = tmp();
            assert!(!load_project_watch(data.path(), "x"));
            save_project_threads(data.path(), "x", 3).unwrap();
            save_project_watch(data.path(), "x", true).unwrap();
            assert!(load_project_watch(data.path(), "x"));
            assert!(!load_project_watch(data.path(), "y"));
            save_project_watch(data.path(), "x", false).unwrap();
            assert!(!load_project_watch(data.path(), "x"));
            assert_eq!(read_json(&data)["projects"]["x"], json!({ "threads": 3 }));
        }

        /// One test per value, as the TypeScript's `it.each`.
        macro_rules! loads_as_off {
            ($($name:ident: $watch:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let data = tmp();
                        write_settings(&data, &json!({ "projects": { "x": { "watch": $watch } } }));
                        assert!(!load_project_watch(data.path(), "x"));
                    }
                )*
            };
        }

        loads_as_off! {
            a_malformed_saved_value_yes_loads_as_off: "yes",
            a_malformed_saved_value_1_loads_as_off: 1,
            a_malformed_saved_value_null_loads_as_off: Value::Null,
        }
    }

    mod thread_count {
        use super::*;

        #[test]
        fn save_then_load_returns_the_count() {
            let data = tmp();
            save_project_threads(data.path(), "x", 8).unwrap();
            assert_eq!(load_project_threads(data.path(), "x"), 8);
        }

        #[test]
        fn each_project_has_its_own_count() {
            let data = tmp();
            save_project_threads(data.path(), "a", 3).unwrap();
            assert_eq!(load_project_threads(data.path(), "a"), 3);
            assert_eq!(load_project_threads(data.path(), "b"), 1);
        }

        #[test]
        fn saving_the_count_keeps_the_other_settings() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            let f = FilterState {
                include: strs(&["smoke"]),
                exclude: vec![],
                hide_non_matching: false,
            };
            let w = WindowState {
                x: 1.0,
                y: 2.0,
                width: 700.0,
                height: 500.0,
                maximized: false,
            };
            let e = EnvSettings {
                env: Some("qa".into()),
                properties: StoredLists {
                    shared: vec![StoredRow::plain("timeout", "5000")],
                    by_env: IndexMap::new(),
                },
            };
            save_opened_project(data.path(), &project).unwrap();
            save_window_state(data.path(), &w).unwrap();
            save_project_filter(data.path(), &project, &f).unwrap();
            save_project_env(data.path(), &project, &e).unwrap();
            save_project_threads(data.path(), &project, 4).unwrap();
            assert_eq!(load_last_project(data.path()), Some(project.clone()));
            assert_eq!(load_window_state(data.path()), Some(w));
            assert_eq!(load_project_filter(data.path(), &project), f);
            assert_eq!(load_project_env(data.path(), &project), e);
            assert_eq!(load_project_threads(data.path(), &project), 4);
        }

        /// One test per value, as the TypeScript's `it.each`.
        macro_rules! loads_as_1 {
            ($($name:ident: $threads:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let data = tmp();
                        write_settings(&data, &json!({ "projects": { "x": { "threads": $threads } } }));
                        assert_eq!(load_project_threads(data.path(), "x"), 1);
                    }
                )*
            };
        }

        loads_as_1! {
            a_malformed_saved_count_the_string_8_loads_as_1: "8",
            a_malformed_saved_count_0_loads_as_1: 0,
            a_malformed_saved_count_2_5_loads_as_1: 2.5,
            a_malformed_saved_count_64_loads_as_1: 64,
            a_malformed_saved_count_null_loads_as_1: Value::Null,
        }

        #[test]
        fn an_unreadable_file_loads_as_1() {
            let data = tmp();
            save_project_threads(data.path(), "x", 8).unwrap();
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_project_threads(data.path(), "x"), 1);
        }
    }

    mod window_state {
        use super::*;

        const OK: WindowState = WindowState {
            x: -1900.0,
            y: 20.0,
            width: 700.0,
            height: 500.0,
            maximized: true,
        };

        #[test]
        fn save_then_load_returns_the_window_state() {
            let data = tmp();
            save_window_state(data.path(), &OK).unwrap();
            assert_eq!(load_window_state(data.path()), Some(OK));
        }

        #[test]
        fn load_window_state_reads_a_broken_state_as_nothing_saved() {
            assert_eq!(load_window_state(tmp().path()), None);
            let no_maximized = json!({ "x": -1900, "y": 20, "width": 700, "height": 500 });
            let windows = [
                json!("abc"),
                json!({ "x": -1900, "y": 20, "width": -5, "height": 500, "maximized": true }),
                json!({ "x": -1900, "y": 20, "width": 0, "height": 500, "maximized": true }),
                json!({ "x": "a", "y": 20, "width": 700, "height": 500, "maximized": true }),
                no_maximized,
            ];
            for window in windows {
                let data = tmp();
                write_settings(&data, &json!({ "window": window }));
                assert_eq!(load_window_state(data.path()), None, "{window}");
            }
            let data = tmp();
            write_text(
                &data,
                r#"{"window":{"x":1e999,"y":0,"width":700,"height":500,"maximized":false}}"#,
            );
            assert_eq!(load_window_state(data.path()), None);
        }

        #[test]
        fn saving_the_window_keeps_the_other_settings() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            let f = FilterState {
                include: strs(&["smoke"]),
                exclude: vec![],
                hide_non_matching: false,
            };
            save_opened_project(data.path(), &project).unwrap();
            save_project_filter(data.path(), &project, &f).unwrap();
            save_window_state(data.path(), &OK).unwrap();
            assert_eq!(load_last_project(data.path()), Some(project.clone()));
            assert_eq!(load_project_filter(data.path(), &project), f);
            assert_eq!(load_window_state(data.path()), Some(OK));
        }

        #[test]
        fn a_failed_rename_leaves_the_file_as_it_was() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            save_opened_project(data.path(), &project).unwrap();
            let before = read_text_of(&data);
            {
                let _refused = hooks::renames_fail(ErrorKind::PermissionDenied);
                let err = save_window_state(data.path(), &OK).unwrap_err();
                assert_eq!(err.kind(), ErrorKind::PermissionDenied);
            }
            assert_eq!(read_text_of(&data), before);
            save_window_state(data.path(), &OK).unwrap();
            assert_eq!(load_last_project(data.path()), Some(project));
            assert_eq!(load_window_state(data.path()), Some(OK));
        }

        #[test]
        fn a_leftover_temp_file_is_ignored() {
            let (data, parent) = (tmp(), tmp());
            let project = folder(&parent, "proj");
            save_opened_project(data.path(), &project).unwrap();
            fs::write(data.path().join("settings.json.tmp"), "{junk").unwrap();
            assert_eq!(load_last_project(data.path()), Some(project.clone()));
            save_window_state(data.path(), &OK).unwrap();
            assert_eq!(load_last_project(data.path()), Some(project));
            assert_eq!(load_window_state(data.path()), Some(OK));
        }

        // Not in the TypeScript: the file holds fractions, and the window state of it reads them.
        #[test]
        fn keeps_fractions() {
            let data = tmp();
            let w = WindowState {
                x: 10.5,
                y: -0.25,
                width: 700.5,
                height: 500.0,
                maximized: false,
            };
            save_window_state(data.path(), &w).unwrap();
            assert_eq!(load_window_state(data.path()), Some(w));
        }
    }

    mod panel_height {
        use super::*;

        #[test]
        fn load_panel_height_reads_a_saved_height() {
            let data = tmp();
            write_settings(&data, &json!({ "panelHeight": 300 }));
            assert_eq!(load_panel_height(data.path()), 300.0);
            write_settings(&data, &json!({ "panelHeight": 2000 }));
            assert_eq!(load_panel_height(data.path()), 2000.0); // clamped when shown, not here
        }

        #[test]
        fn load_panel_height_falls_back_to_the_default() {
            assert_eq!(load_panel_height(tmp().path()), 192.0);
            for v in [json!("abc"), json!(-5), json!(50), json!(null), json!("300")] {
                let data = tmp();
                write_settings(&data, &json!({ "panelHeight": v }));
                assert_eq!(load_panel_height(data.path()), 192.0, "{v}");
            }
            let data = tmp();
            write_text(&data, r#"{"panelHeight":1e999}"#);
            assert_eq!(load_panel_height(data.path()), 192.0);
            let data = tmp();
            save_panel_height(data.path(), 300.0).unwrap();
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_panel_height(data.path()), 192.0);
        }

        #[test]
        fn save_panel_height_writes_a_rounded_integer_and_keeps_the_rest() {
            let data = tmp();
            write_settings(&data, &json!({ "lastProject": "D:\\x" }));
            save_panel_height(data.path(), 300.4).unwrap();
            let saved: Value = serde_json::from_str(&fs::read_to_string(settings_file(&data)).unwrap()).unwrap();
            assert_eq!(saved, json!({ "lastProject": "D:\\x", "panelHeight": 300 }));
        }
    }

    mod panel_tabs {
        use super::*;
        use crate::panel_tabs::TabId::{Console, History, Http};

        #[test]
        fn save_then_load_returns_the_order_as_a_json_list() {
            let data = tmp();
            assert_eq!(load_tab_order(data.path()), [Console, History, Http]);
            save_tab_order(data.path(), &[History, Http, Console]).unwrap();
            assert_eq!(load_tab_order(data.path()), [History, Http, Console]);
            let saved: Value = serde_json::from_str(&fs::read_to_string(settings_file(&data)).unwrap()).unwrap();
            assert_eq!(saved, json!({ "panelTabs": ["history", "http", "console"] }));
        }

        #[test]
        fn falls_back_to_the_default_when_the_file_cannot_be_read() {
            let data = tmp();
            save_tab_order(data.path(), &[History, Http, Console]).unwrap();
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_tab_order(data.path()), [Console, History, Http]);
        }
    }

    mod update_settings {
        use super::*;

        fn version(text: &str) -> Option<Version> {
            Some(parse_version(text).unwrap())
        }

        #[test]
        fn reads_missing_or_malformed_update_settings_as_defaults_r12() {
            let dir = tmp();
            write_settings(
                &dir,
                &json!({
                    "updates": {
                        "channel": "beta",
                        "checkAtStartup": "yes",
                        "autoUpdate": true,
                        "skipped": "v1",
                        "failed": 3,
                        "lastChecked": 5,
                    },
                }),
            );
            assert_eq!(
                load_update_settings(dir.path()),
                UpdateSettings {
                    channel: None,
                    check_at_startup: true,
                    auto_update: true,
                    skipped: None,
                    failed: None,
                    last_checked: None,
                }
            );
        }

        #[test]
        fn reads_no_file_a_corrupt_file_or_a_non_object_key_as_the_defaults() {
            assert_eq!(load_update_settings(tmp().path()), UpdateSettings::default());
            let corrupt = tmp();
            write_text(&corrupt, "{not json");
            assert_eq!(load_update_settings(corrupt.path()), UpdateSettings::default());
            for updates in [json!(null), json!("x"), json!(7), json!(["stable"])] {
                let dir = tmp();
                write_settings(&dir, &json!({ "updates": updates }));
                assert_eq!(load_update_settings(dir.path()), UpdateSettings::default(), "{updates}");
            }
        }

        #[test]
        fn reads_an_unreadable_file_as_the_defaults_and_never_throws() {
            let dir = tmp();
            write_settings(&dir, &json!({ "updates": { "autoUpdate": true } }));
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_update_settings(dir.path()), UpdateSettings::default());
        }

        #[test]
        fn keeps_valid_fields_and_defaults_only_the_bad_ones() {
            let dir = tmp();
            write_settings(
                &dir,
                &json!({
                    "updates": {
                        "channel": "stable",
                        "checkAtStartup": false,
                        "autoUpdate": "no",
                        "skipped": "1.0.1",
                        "failed": "1.0.2-rc.1",
                        "lastChecked": "2026-10-28T10:00:00Z",
                    },
                }),
            );
            assert_eq!(
                load_update_settings(dir.path()),
                UpdateSettings {
                    channel: Some(Channel::Stable),
                    check_at_startup: false,
                    auto_update: false,
                    skipped: version("1.0.1"),
                    failed: version("1.0.2-rc.1"),
                    last_checked: Some("2026-10-28T10:00:00Z".into()),
                }
            );
        }

        #[test]
        fn reads_a_skipped_or_failed_version_that_is_not_valid_as_null_so_compare_versions_never_throws() {
            for bad in ["v1.0.1", "1.0", "1.0.0-beta.1", "01.0.0", "", " 1.0.0"] {
                let dir = tmp();
                write_settings(&dir, &json!({ "updates": { "skipped": bad, "failed": bad } }));
                let s = load_update_settings(dir.path());
                assert_eq!(s.skipped, None, "{bad}");
                assert_eq!(s.failed, None, "{bad}");
            }
        }

        #[test]
        fn reads_a_last_checked_that_is_not_a_date_as_null() {
            let dir = tmp();
            write_settings(&dir, &json!({ "updates": { "lastChecked": "yesterday" } }));
            assert_eq!(load_update_settings(dir.path()).last_checked, None);
        }

        #[test]
        fn reads_no_saved_channel_as_not_picked_and_saves_one_only_once_picked_r1_3() {
            let dir = tmp();
            assert_eq!(load_update_settings(dir.path()).channel, None);
            let checked = UpdateSettings {
                last_checked: Some("2026-10-28T10:00:00Z".into()),
                ..UpdateSettings::default()
            };
            save_update_settings(dir.path(), &checked).unwrap();
            assert!(read_json(&dir)["updates"].get("channel").is_none());
            assert_eq!(load_update_settings(dir.path()).channel, None);
            let stable = UpdateSettings {
                channel: Some(Channel::Stable),
                ..UpdateSettings::default()
            };
            save_update_settings(dir.path(), &stable).unwrap();
            assert_eq!(read_json(&dir)["updates"]["channel"], "stable");
            assert_eq!(load_update_settings(dir.path()).channel, Some(Channel::Stable));
        }

        #[test]
        fn saves_update_settings_and_keeps_the_other_keys() {
            let dir = tmp();
            write_settings(&dir, &json!({ "lastProject": abs("p") }));
            let skipped = UpdateSettings {
                skipped: version("1.0.1"),
                ..UpdateSettings::default()
            };
            save_update_settings(dir.path(), &skipped).unwrap();
            assert_eq!(read_json(&dir)["lastProject"], json!(abs("p")));
            assert_eq!(load_update_settings(dir.path()).skipped, version("1.0.1"));
        }

        #[test]
        fn saving_other_settings_keeps_the_update_settings() {
            let dir = tmp();
            let s = UpdateSettings {
                auto_update: true,
                failed: version("1.0.3"),
                ..UpdateSettings::default()
            };
            save_update_settings(dir.path(), &s).unwrap();
            let window = WindowState {
                x: 0.0,
                y: 0.0,
                width: 700.0,
                height: 500.0,
                maximized: false,
            };
            save_window_state(dir.path(), &window).unwrap();
            assert_eq!(load_update_settings(dir.path()), s);
        }

        // Not in the TypeScript: `Date.parse` is what reads `lastChecked`; this reads RFC 3339, which Grace writes.
        #[test]
        fn reads_a_last_checked_in_rfc_3339_with_a_zone_or_milliseconds() {
            for date in ["2026-10-28T10:00:00.123Z", "2026-10-28T12:00:00+02:00"] {
                assert_eq!(
                    parse_update_settings(&json!({ "lastChecked": date }))
                        .last_checked
                        .as_deref(),
                    Some(date)
                );
            }
            for date in ["2026-13-28T10:00:00Z", "", "28 Oct 2026"] {
                assert_eq!(
                    parse_update_settings(&json!({ "lastChecked": date })).last_checked,
                    None,
                    "{date}"
                );
            }
        }
    }

    mod appearance {
        use super::*;

        #[test]
        fn reads_no_file_a_corrupt_file_or_an_unreadable_one_as_follow_windows() {
            assert_eq!(load_appearance(tmp().path()), Theme::System);
            let corrupt = tmp();
            write_text(&corrupt, "{not json");
            assert_eq!(load_appearance(corrupt.path()), Theme::System);
            let busy = tmp();
            write_settings(&busy, &json!({ "appearance": { "theme": "dark" } }));
            let _busy = hooks::reads_fail(ErrorKind::ResourceBusy);
            assert_eq!(load_appearance(busy.path()), Theme::System);
        }

        #[test]
        fn reads_a_theme_that_is_not_one_of_the_three_or_a_key_that_is_not_an_object_as_follow_windows() {
            let appearances = [
                json!({ "theme": "purple" }),
                json!({ "theme": 3 }),
                json!({}),
                json!(null),
                json!("dark"),
                json!(["dark"]),
            ];
            for appearance in appearances {
                let dir = tmp();
                write_settings(&dir, &json!({ "appearance": appearance }));
                assert_eq!(load_appearance(dir.path()), Theme::System, "{appearance}");
            }
        }

        #[test]
        fn saves_a_theme_keeps_the_other_keys_and_loads_it_back() {
            let dir = tmp();
            write_settings(&dir, &json!({ "lastProject": abs("p") }));
            save_appearance(dir.path(), Theme::Dark).unwrap();
            assert_eq!(
                read_json(&dir),
                json!({ "lastProject": abs("p"), "appearance": { "theme": "dark" } })
            );
            assert_eq!(load_appearance(dir.path()), Theme::Dark);
            save_appearance(dir.path(), Theme::Light).unwrap();
            assert_eq!(load_appearance(dir.path()), Theme::Light);
            let window = WindowState {
                x: 0.0,
                y: 0.0,
                width: 700.0,
                height: 500.0,
                maximized: false,
            };
            save_window_state(dir.path(), &window).unwrap();
            assert_eq!(load_appearance(dir.path()), Theme::Light);
        }
    }

    // ----- not in the TypeScript: the file's text, and what the port adds -----

    mod the_file {
        use super::*;

        /// The text the TypeScript's `update` leaves after each of these saves, as `JSON.stringify` writes it (the same
        /// saves, run on JavaScript objects).
        const EXPECTED: [&str; 8] = [
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"]}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true}}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true,"env":"qa","properties":{"shared":[{"name":"timeout","value":"5000"},{"name":"APP_X","secret":"c2VjcmV0","envVar":true}],"byEnv":{"2":[],"10":[{"name":"n","value":"1"}],"qa":[{"name":"timeout","value":"9000"}],"dev":[]}}}}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true,"env":"qa","properties":{"shared":[{"name":"timeout","value":"5000"},{"name":"APP_X","secret":"c2VjcmV0","envVar":true}],"byEnv":{"2":[],"10":[{"name":"n","value":"1"}],"qa":[{"name":"timeout","value":"9000"}],"dev":[]}},"threads":4,"watch":true}}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true,"env":"qa","properties":{"shared":[{"name":"timeout","value":"5000"},{"name":"APP_X","secret":"c2VjcmV0","envVar":true}],"byEnv":{"2":[],"10":[{"name":"n","value":"1"}],"qa":[{"name":"timeout","value":"9000"}],"dev":[]}},"threads":4},"proj-two":{"threads":2}}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true,"env":"qa","properties":{"shared":[{"name":"timeout","value":"5000"},{"name":"APP_X","secret":"c2VjcmV0","envVar":true}],"byEnv":{"2":[],"10":[{"name":"n","value":"1"}],"qa":[{"name":"timeout","value":"9000"}],"dev":[]}},"threads":4},"proj-two":{"threads":2}},"updates":{"channel":"stable","checkAtStartup":true,"autoUpdate":false,"skipped":"1.0.1","failed":null,"lastChecked":"2026-10-28T10:00:00.000Z"}}"#,
            r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/proj","recentProjects":["/d/proj"],"window":{"x":-8,"y":0,"width":1936,"height":1056.5,"maximized":false},"projects":{"proj-one":{"include":["smoke"],"exclude":["wip"],"hideNonMatching":true,"env":"qa","properties":{"shared":[{"name":"timeout","value":"5000"},{"name":"APP_X","secret":"c2VjcmV0","envVar":true}],"byEnv":{"2":[],"10":[{"name":"n","value":"1"}],"qa":[{"name":"timeout","value":"9000"}],"dev":[]}},"threads":4},"proj-two":{"threads":2}},"updates":{"channel":"stable","checkAtStartup":true,"autoUpdate":false,"skipped":"1.0.1","failed":null,"lastChecked":"2026-10-28T10:00:00.000Z"},"appearance":{"theme":"dark"}}"#,
        ];

        #[test]
        fn writes_the_text_the_typescript_writes() {
            let data = tmp();
            let d = data.path();
            // a key of a newer Grace is kept, and so is the position of a key that is saved again
            write_text(&data, r#"{"future":{"a":[1,2.5,null]},"lastProject":"/d/old"}"#);
            let mut steps = EXPECTED.iter();
            let mut expect = |data: &TempDir| assert_eq!(read_text_of(data), *steps.next().unwrap());

            save_opened_project(d, "/d/proj").unwrap();
            expect(&data);
            let window = WindowState {
                x: -8.0,
                y: 0.0,
                width: 1936.0,
                height: 1056.5,
                maximized: false,
            };
            save_window_state(d, &window).unwrap();
            expect(&data);
            let filter = FilterState {
                include: strs(&["smoke"]),
                exclude: strs(&["wip"]),
                hide_non_matching: true,
            };
            save_project_filter(d, "proj-one", &filter).unwrap();
            expect(&data);
            let env = EnvSettings {
                env: Some("qa".into()),
                properties: StoredLists {
                    shared: vec![
                        StoredRow::plain("timeout", "5000"),
                        StoredRow::secret("APP_X", "c2VjcmV0").as_env_var(),
                    ],
                    // as the window gave them; the file lists the integer-like names first, as JavaScript does
                    by_env: IndexMap::from([
                        ("qa".to_string(), vec![StoredRow::plain("timeout", "9000")]),
                        ("dev".to_string(), vec![]),
                        ("10".to_string(), vec![StoredRow::plain("n", "1")]),
                        ("2".to_string(), vec![]),
                    ]),
                },
            };
            save_project_env(d, "proj-one", &env).unwrap();
            expect(&data);
            save_project_threads(d, "proj-one", 4).unwrap();
            save_project_watch(d, "proj-one", true).unwrap();
            expect(&data);
            save_project_watch(d, "proj-one", false).unwrap();
            save_project_threads(d, "proj-two", 2).unwrap();
            expect(&data);
            let updates = UpdateSettings {
                channel: Some(Channel::Stable),
                skipped: parse_version("1.0.1"),
                last_checked: Some("2026-10-28T10:00:00.000Z".into()),
                ..UpdateSettings::default()
            };
            save_update_settings(d, &updates).unwrap();
            expect(&data);
            save_appearance(d, Theme::Dark).unwrap();
            expect(&data);
            assert!(steps.next().is_none());
            // and what is written is what is read
            assert_eq!(
                load_project_env(d, "proj-one")
                    .properties
                    .by_env
                    .keys()
                    .collect::<Vec<_>>(),
                ["2", "10", "qa", "dev"]
            );
        }

        #[test]
        fn an_environment_named_by_digits_is_listed_first_whatever_the_order_of_the_file() {
            let data = tmp();
            write_text(
                &data,
                r#"{"projects":{"x":{"properties":{"shared":[],"byEnv":{"qa":[],"10":[],"dev":[],"2":[]}}}}}"#,
            );
            let envs = load_project_env(data.path(), "x").properties.by_env;
            assert_eq!(envs.keys().collect::<Vec<_>>(), ["2", "10", "qa", "dev"]);
            assert_eq!(skipped_properties(data.path(), "x"), Vec::<String>::new());
        }

        #[test]
        fn a_failed_save_leaves_no_temp_file() {
            let data = tmp();
            save_appearance(data.path(), Theme::Dark).unwrap();
            {
                let _refused = hooks::renames_fail(ErrorKind::PermissionDenied);
                assert!(save_appearance(data.path(), Theme::Light).is_err());
            }
            assert!(!data.path().join("settings.json.tmp").exists());
            assert_eq!(load_appearance(data.path()), Theme::Dark);
            save_appearance(data.path(), Theme::Light).unwrap();
            assert!(!data.path().join("settings.json.tmp").exists());
        }

        #[test]
        fn saves_from_two_threads_lose_neither() {
            let data = tmp();
            thread::scope(|scope| {
                for t in 0..4 {
                    let dir = data.path();
                    scope.spawn(move || {
                        for i in 0..5 {
                            save_project_threads(dir, &format!("p{t}-{i}"), 2).unwrap();
                        }
                    });
                }
            });
            for t in 0..4 {
                for i in 0..5 {
                    assert_eq!(load_project_threads(data.path(), &format!("p{t}-{i}")), 2);
                }
            }
            assert_eq!(read_json(&data)["projects"].as_object().unwrap().len(), 20);
        }

        #[test]
        fn creates_the_data_folder() {
            let parent = tmp();
            let data = parent.path().join("a").join("Grace");
            save_appearance(&data, Theme::Dark).unwrap();
            assert!(data.join("settings.json").is_file());
        }

        #[test]
        fn a_file_that_is_not_utf8_reads_as_node_reads_it() {
            let data = tmp();
            // a stray byte in a value, as `readFileSync(.., "utf8")` turns it into U+FFFD, leaves the JSON whole
            fs::write(
                settings_file(&data),
                b"{\"appearance\":{\"theme\":\"dark\"},\"note\":\"caf\xe9\"}",
            )
            .unwrap();
            assert_eq!(load_appearance(data.path()), Theme::Dark);
            save_appearance(data.path(), Theme::Light).unwrap();
            assert_eq!(read_json(&data)["note"], "caf\u{fffd}");
        }

        #[test]
        fn a_value_it_does_not_own_is_kept_as_it_was() {
            let data = tmp();
            write_text(
                &data,
                r#"{"future":{"b":true,"a":[{"z":1,"y":[]}],"2":0,"1":1},"x":"é\n"}"#,
            );
            save_appearance(data.path(), Theme::Dark).unwrap();
            // `JSON.parse` lists the array indexes of an object first, then `JSON.stringify` writes them so
            assert_eq!(
                read_text_of(&data),
                r#"{"future":{"1":1,"2":0,"b":true,"a":[{"z":1,"y":[]}]},"x":"é\n","appearance":{"theme":"dark"}}"#
            );
            // and `1.0` as `1`
            write_text(&data, r#"{"a":1.0,"b":-0,"c":2.5,"d":1e3,"e":[3.0]}"#);
            save_appearance(data.path(), Theme::Dark).unwrap();
            assert_eq!(
                read_text_of(&data),
                r#"{"a":1,"b":0,"c":2.5,"d":1000,"e":[3],"appearance":{"theme":"dark"}}"#
            );
        }

        #[test]
        fn a_stored_row_is_written_with_name_then_its_value_then_its_kind() {
            let rows = [
                StoredRow::plain("a", "1"),
                StoredRow::secret("b", "c2VjcmV0"),
                StoredRow::plain("C", "2").as_env_var(),
                StoredRow::secret("D", "x").as_env_var(),
            ];
            let text: Vec<String> = rows.iter().map(|r| r.to_json().to_string()).collect();
            assert_eq!(
                text,
                [
                    r#"{"name":"a","value":"1"}"#,
                    r#"{"name":"b","secret":"c2VjcmV0"}"#,
                    r#"{"name":"C","value":"2","envVar":true}"#,
                    r#"{"name":"D","secret":"x","envVar":true}"#,
                ]
            );
        }
    }
}
