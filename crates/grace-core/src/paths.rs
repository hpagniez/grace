//! Where Grace keeps its data. The folder stays where Electron Grace keeps it (ADR 0031): `%APPDATA%\Grace` on
//! Windows, `$XDG_CONFIG_HOME/Grace` (else `~/.config/Grace`) on Linux. `GRACE_USER_DATA` overrides it, as
//! `src/main.ts` lets the tests do.
use std::path::PathBuf;

/// The data folder, from the environment `var` gives: `GRACE_USER_DATA` first, then the platform's folder.
/// None only when the environment names no home at all.
pub fn data_dir_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let set = |name: &str| var(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    if let Some(dir) = set("GRACE_USER_DATA") {
        return Some(dir);
    }
    let base = if cfg!(windows) {
        set("APPDATA")?
    } else {
        set("XDG_CONFIG_HOME").or_else(|| set("HOME").map(|h| h.join(".config")))?
    };
    Some(base.join("Grace"))
}

/// The data folder of this process.
pub fn data_dir() -> Option<PathBuf> {
    data_dir_from(|name| std::env::var(name).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
    }

    #[test]
    fn grace_user_data_wins() {
        let vars = [
            ("GRACE_USER_DATA", "/tmp/grace-test"),
            ("APPDATA", "/a"),
            ("HOME", "/h"),
        ];
        assert_eq!(data_dir_from(env(&vars)), Some(PathBuf::from("/tmp/grace-test")));
    }

    #[test]
    fn an_empty_grace_user_data_is_ignored() {
        let vars = [("GRACE_USER_DATA", ""), ("APPDATA", "/a"), ("XDG_CONFIG_HOME", "/x")];
        let expected = if cfg!(windows) { "/a" } else { "/x" };
        assert_eq!(data_dir_from(env(&vars)), Some(PathBuf::from(expected).join("Grace")));
    }

    #[cfg(windows)]
    #[test]
    fn is_grace_under_appdata_on_windows() {
        let vars = [("APPDATA", r"C:\Users\h\AppData\Roaming")];
        assert_eq!(
            data_dir_from(env(&vars)),
            Some(PathBuf::from(r"C:\Users\h\AppData\Roaming\Grace"))
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn is_grace_under_the_config_folder_on_linux() {
        assert_eq!(
            data_dir_from(env(&[("HOME", "/home/h")])),
            Some(PathBuf::from("/home/h/.config/Grace"))
        );
        let vars = [("HOME", "/home/h"), ("XDG_CONFIG_HOME", "/cfg")];
        assert_eq!(data_dir_from(env(&vars)), Some(PathBuf::from("/cfg/Grace")));
        assert_eq!(data_dir_from(env(&[])), None);
    }
}
