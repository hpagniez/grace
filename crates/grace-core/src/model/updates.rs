//! `src/shared/updates.ts`: releases, channels and update settings (docs/specs/2026-10-28-app-updates.md R1, R4, R5,
//! R12).
use super::js;
use super::version::{Version, parse_version};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    Stable,
    Prerelease,
}

/// The first release that ships the updater; older ones cannot be installed from the app.
pub const FIRST_UPDATER: &str = "1.0.0-rc.4";
const FIRST_UPDATER_VERSION: Version = Version::new(1, 0, 0, Some(4));

pub const RELEASES_API: &str = "https://api.github.com/repos/hpagniez/grace/releases?per_page=100";

/// Written into `resources` by the Forge hook: the packaged folder's top-level names.
pub const APP_FILES: &str = "app-files.json";

/// The names and links below take the text of a validated version, or the `Version` itself.
pub fn release_page(v: impl Display) -> String {
    format!("https://github.com/hpagniez/grace/releases/tag/v{v}")
}

pub fn zip_name(v: impl Display) -> String {
    format!("Grace-win32-x64-{v}.zip")
}

pub fn download_prefix(v: impl Display) -> String {
    format!("https://github.com/hpagniez/grace/releases/download/v{v}/")
}

/// An installable release (R1.4), reduced to the fields Grace uses.
///
/// Differs from the TypeScript in `version`, a `Version` where it is a string: a release is only made of a version that
/// parses, so every comparison is infallible. It is written to and read from JSON as the same text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: Version,
    pub prerelease: bool,
    /// The release's `published_at` when it is a date, else the empty string.
    pub published_at: String,
    pub url: String,
    pub size: u64,
    /// SHA-256 as lowercase hex.
    pub sha256: String,
}

/// A SHA-256 as the GitHub API writes a digest: `sha256:` and 64 lowercase hex digits. Gives the digits.
fn sha256_of(digest: &str) -> Option<&str> {
    let hex = digest.strip_prefix("sha256:")?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))).then_some(hex)
}

/// Whether `Date.parse` would read the text. GitHub writes RFC 3339 (`2026-10-28T10:00:00Z`), which is all this reads;
/// `Date.parse` also falls back to whatever format the engine knows, which GitHub does not send.
fn is_date(s: &str) -> bool {
    DateTime::parse_from_rfc3339(s).is_ok()
}

/// One GitHub release entry, checked field by field (R1.4); `None` when it is not installable.
fn to_release(e: &Value) -> Option<Release> {
    let o = e.as_object()?;
    if o.get("draft") != Some(&Value::Bool(false)) {
        return None;
    }
    let prerelease = o.get("prerelease")?.as_bool()?;
    let text = o.get("tag_name")?.as_str()?.strip_prefix('v')?;
    let assets = o.get("assets")?.as_array()?;
    let version = parse_version(text).filter(|v| *v >= FIRST_UPDATER_VERSION)?;
    let name = zip_name(version);
    let mut zips = assets
        .iter()
        .filter_map(Value::as_object)
        .filter(|a| a.get("name").and_then(Value::as_str) == Some(&name));
    let zip = zips.next()?;
    if zips.next().is_some() {
        return None;
    }
    let size = js::integer(zip.get("size")?).filter(|n| *n > 0.0 && *n < 18_446_744_073_709_551_616.0)? as u64;
    let sha256 = sha256_of(zip.get("digest")?.as_str()?)?;
    // exactly: a prefix would let `/../` take the download elsewhere
    let url = zip
        .get("browser_download_url")?
        .as_str()
        .filter(|u| *u == download_prefix(version) + &name)?;
    let published_at = o
        .get("published_at")
        .and_then(Value::as_str)
        .filter(|d| is_date(d))
        .unwrap_or("");
    Some(Release {
        version,
        prerelease,
        published_at: published_at.to_string(),
        url: url.to_string(),
        size,
        sha256: sha256.to_string(),
    })
}

/// The installable releases in a releases API response, newest first, one per version (R1.4).
pub fn parse_releases(json: &Value) -> Vec<Release> {
    let Some(entries) = json.as_array() else {
        return vec![];
    };
    let mut rs: Vec<Release> = entries.iter().filter_map(to_release).collect();
    // stable, as JavaScript's `sort`: of two entries of one version, the first stays first
    rs.sort_by_key(|r| std::cmp::Reverse(r.version));
    rs.dedup_by_key(|r| r.version);
    rs
}

/// The releases a channel offers: stable hides pre-releases.
pub fn on_channel(rs: &[Release], c: Channel) -> impl Iterator<Item = &Release> {
    rs.iter().filter(move |r| c == Channel::Prerelease || !r.prerelease)
}

/// The highest version on the channel (R1.5), or `None`. Of equal versions, the first.
pub fn latest_on(rs: &[Release], c: Channel) -> Option<&Release> {
    on_channel(rs, c).reduce(|best, r| if r.version > best.version { r } else { best })
}

/// The update settings of `settings.json`.
///
/// Differs from the TypeScript in `skipped` and `failed`, `Version`s where they are strings: they are only ever set
/// to a valid version, and the settings file's reader reads one that is not as `None` (as Electron Grace's does).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    /// None until the person picks one, and never saved till then: see `channel_of` (R1.3).
    pub channel: Option<Channel>,
    pub check_at_startup: bool,
    pub auto_update: bool,
    pub skipped: Option<Version>,
    pub failed: Option<Version>,
    pub last_checked: Option<String>,
}

/// Settings before the person changes any (R12): check at startup, never update by itself, remember nothing.
///
/// The TypeScript's `defaultUpdateSettings()`.
impl Default for UpdateSettings {
    fn default() -> Self {
        UpdateSettings {
            channel: None,
            check_at_startup: true,
            auto_update: false,
            skipped: None,
            failed: None,
            last_checked: None,
        }
    }
}

/// R1.3: the channel picked, else pre-release while a pre-release runs and stable otherwise.
pub fn channel_of(s: &UpdateSettings, running: &Version) -> Channel {
    s.channel.unwrap_or(if running.is_prerelease() {
        Channel::Prerelease
    } else {
        Channel::Stable
    })
}

/// The release the startup modal offers: out of date, auto-update off, above the skipped one (R4.1).
///
/// `running` is the version of the app: a `Version`, so no comparison here can fail (the TypeScript's throws on a
/// version that does not parse).
pub fn prompt_for<'a>(rs: &'a [Release], s: &UpdateSettings, running: &Version) -> Option<&'a Release> {
    let latest = latest_on(rs, channel_of(s, running))?;
    if s.auto_update || latest.version <= *running {
        return None;
    }
    match &s.skipped {
        Some(skipped) if latest.version <= *skipped => None,
        _ => Some(latest),
    }
}

/// The release auto-update installs at startup: skipped counts for nothing, failed is never retried (R5.2).
pub fn auto_update_to<'a>(rs: &'a [Release], s: &UpdateSettings, running: &Version) -> Option<&'a Release> {
    let latest = latest_on(rs, channel_of(s, running))?;
    if !s.auto_update || latest.version <= *running {
        return None;
    }
    (s.failed != Some(latest.version)).then_some(latest)
}

/// A saved copy of the data of one version (R9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub version: String,
    pub saved_at: String,
}

/// What the window shows about updates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    pub running: String,
    pub packaged: bool,
    pub settings: UpdateSettings,
    pub releases: Option<Vec<Release>>,
    pub checking: bool,
    pub error: Option<String>,
    pub backups: Vec<Backup>,
}

/// A step of an install (R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateStep {
    Download,
    Check,
    Unpack,
    /// The hand-over to the switch script, which cannot be cancelled (R8.5).
    Restart,
}

/// Install progress, reported by the updater to the window (R8): a step and its bytes, or the error that ended it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpdateProgress {
    Step {
        version: String,
        rollback: bool,
        step: UpdateStep,
        received: u64,
        total: u64,
    },
    Error {
        version: String,
        rollback: bool,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::version::is_prerelease;
    use serde_json::json;

    fn v(version: &str) -> Version {
        parse_version(version).unwrap()
    }

    /// What `gh` varies of a GitHub API release entry; a default one is installable.
    #[derive(Default)]
    struct Gh {
        prerelease: Option<bool>,
        draft: Option<bool>,
        assets: Option<Vec<Value>>,
        digest: Option<String>,
        url: Option<String>,
    }

    /// A GitHub API release entry; by default one that is installable.
    fn gh(tag: &str, o: Gh) -> Value {
        let version = &tag[1..];
        json!({
            "tag_name": tag,
            "draft": o.draft.unwrap_or(false),
            "prerelease": o.prerelease.unwrap_or_else(|| is_prerelease(version)),
            "published_at": "2026-10-28T10:00:00Z",
            "assets": o.assets.unwrap_or_else(|| vec![json!({
                "name": zip_name(version),
                "size": 218_000_000,
                "digest": o.digest.unwrap_or_else(|| format!("sha256:{}", "ab".repeat(32))),
                "browser_download_url": o.url.unwrap_or_else(|| format!("{}{}", download_prefix(version), zip_name(version))),
            })]),
        })
    }

    fn ok(tag: &str) -> Value {
        gh(tag, Gh::default())
    }

    fn r(version: &str) -> Release {
        r_on(version, is_prerelease(version))
    }

    fn r_on(version: &str, prerelease: bool) -> Release {
        Release {
            version: v(version),
            prerelease,
            published_at: "2026-10-28T10:00:00Z".into(),
            url: format!("{}{}", download_prefix(version), zip_name(version)),
            size: 218_000_000,
            sha256: "ab".repeat(32),
        }
    }

    fn versions<'a>(rs: impl IntoIterator<Item = &'a Release>) -> Vec<String> {
        rs.into_iter().map(|x| x.version.to_string()).collect()
    }

    fn rs() -> Vec<Release> {
        parse_releases(&json!([ok("v1.0.0-rc.5"), ok("v1.0.0-rc.6")]))
    }

    mod names_and_links {
        use super::*;

        #[test]
        fn builds_them_from_a_validated_version() {
            assert_eq!(zip_name("1.0.0-rc.5"), "Grace-win32-x64-1.0.0-rc.5.zip");
            assert_eq!(
                download_prefix("1.0.0-rc.5"),
                "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/"
            );
            assert_eq!(
                release_page("1.0.0-rc.5"),
                "https://github.com/hpagniez/grace/releases/tag/v1.0.0-rc.5"
            );
            // a `Version` gives the same text as the string
            assert_eq!(zip_name(v("1.0.0-rc.5")), zip_name("1.0.0-rc.5"));
        }
    }

    mod parse_releases {
        use super::*;

        #[test]
        fn keeps_installable_releases_only_r1_4() {
            let parsed = parse_releases(&json!([
                ok("v1.0.0-rc.5"),
                ok("v1.0.0-rc.3"),
                gh(
                    "v1.0.0-rc.6",
                    Gh {
                        draft: Some(true),
                        ..Gh::default()
                    }
                ),
                ok("v2.0.0-beta"),
                gh(
                    "v1.0.0-rc.7",
                    Gh {
                        assets: Some(vec![]),
                        ..Gh::default()
                    }
                ),
                gh(
                    "v1.0.0-rc.8",
                    Gh {
                        digest: Some("md5:abc".into()),
                        ..Gh::default()
                    }
                ),
                gh(
                    "v1.0.0-rc.9",
                    Gh {
                        url: Some("https://evil.example/Grace-win32-x64-1.0.0-rc.9.zip".into()),
                        ..Gh::default()
                    }
                ),
            ]));
            assert_eq!(versions(&parsed), ["1.0.0-rc.5"]); // rc.3 is below FIRST_UPDATER
        }

        #[test]
        fn reads_the_fields_grace_uses() {
            assert_eq!(
                rs()[1],
                Release {
                    version: v("1.0.0-rc.5"),
                    prerelease: true,
                    published_at: "2026-10-28T10:00:00Z".into(),
                    url:
                        "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/Grace-win32-x64-1.0.0-rc.5.zip"
                            .into(),
                    size: 218_000_000,
                    sha256: "ab".repeat(32),
                }
            );
        }

        #[test]
        fn lists_newest_first_one_per_version() {
            let parsed = parse_releases(&json!([
                ok("v1.0.0-rc.5"),
                ok("v1.0.0-rc.10"),
                ok("v1.0.0-rc.5"),
                ok("v1.0.0-rc.9"),
                gh(
                    "v1.0.0",
                    Gh {
                        prerelease: Some(false),
                        ..Gh::default()
                    }
                ),
            ]));
            assert_eq!(versions(&parsed), ["1.0.0", "1.0.0-rc.10", "1.0.0-rc.9", "1.0.0-rc.5"]);
        }

        #[test]
        fn rejects_an_entry_that_fails_any_one_field() {
            let entry = ok("v1.0.0-rc.5");
            let zip = entry["assets"][0].clone();
            let set = |key: &str, value: Value| {
                let mut e = entry.clone();
                e[key] = value;
                e
            };
            let drop = |key: &str| {
                let mut e = entry.clone();
                e.as_object_mut().unwrap().remove(key);
                e
            };
            let asset = |key: &str, value: Option<Value>| {
                let mut z = zip.clone();
                match value {
                    Some(value) => z[key] = value,
                    None => drop_key(&mut z, key),
                }
                set("assets", json!([z]))
            };
            fn drop_key(z: &mut Value, key: &str) {
                z.as_object_mut().unwrap().remove(key);
            }
            let prefix = download_prefix("1.0.0-rc.5");
            let bad = [
                set("tag_name", json!("1.0.0-rc.5")),
                drop("draft"),
                drop("prerelease"),
                set("assets", json!("zip")),
                set("assets", json!([zip, zip])),
                asset("name", Some(json!("Grace-win32-x64-1.0.0-rc.6.zip"))),
                asset("size", Some(json!(0))),
                asset("size", Some(json!("218"))),
                asset("digest", Some(json!(format!("sha256:{}", "AB".repeat(32))))),
                asset("digest", Some(json!("sha256:abc"))),
                asset("browser_download_url", None),
                gh(
                    "v1.0.0-rc.5",
                    Gh {
                        url: Some("http://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/x.zip".into()),
                        ..Gh::default()
                    },
                ),
                // the right prefix, then out of it
                gh(
                    "v1.0.0-rc.5",
                    Gh {
                        url: Some(format!(
                            "{prefix}../../../../evil/grace/releases/download/v1.0.0-rc.5/{}",
                            zip_name("1.0.0-rc.5")
                        )),
                        ..Gh::default()
                    },
                ),
                gh(
                    "v1.0.0-rc.5",
                    Gh {
                        url: Some(format!("{prefix}other.zip")),
                        ..Gh::default()
                    },
                ),
            ];
            for b in bad {
                assert_eq!(parse_releases(&json!([b.clone()])), vec![], "{b}");
            }
        }

        #[test]
        fn ignores_other_assets_next_to_the_zip() {
            let mut e = ok("v1.0.0-rc.5");
            e["assets"]
                .as_array_mut()
                .unwrap()
                .push(json!({ "name": "notes.txt", "size": 5, "digest": "sha256:x", "browser_download_url": "x" }));
            assert_eq!(versions(&parse_releases(&json!([e]))), ["1.0.0-rc.5"]);
        }

        #[test]
        fn ignores_anything_that_is_not_an_array_of_objects() {
            for x in [Value::Null, json!({}), json!("<html>"), json!([1, "a", null])] {
                assert_eq!(parse_releases(&x), vec![], "{x}");
            }
        }

        /// Not in the TypeScript: `publishedAt` is empty when the date is none, and the first updater is what its text
        /// says.
        #[test]
        fn leaves_published_at_empty_for_a_date_that_is_none() {
            let mut e = ok("v1.0.0-rc.5");
            e["published_at"] = json!("last week");
            assert_eq!(parse_releases(&json!([e.clone()]))[0].published_at, "");
            e["published_at"] = Value::Null;
            assert_eq!(parse_releases(&json!([e.clone()]))[0].published_at, "");
            e.as_object_mut().unwrap().remove("published_at");
            assert_eq!(parse_releases(&json!([e]))[0].published_at, "");
            assert_eq!(parse_version(FIRST_UPDATER), Some(FIRST_UPDATER_VERSION));
        }

        /// Not in the TypeScript: a release is written as the text the window reads.
        #[test]
        fn writes_a_release_as_the_typescript_does() {
            assert_eq!(
                serde_json::to_value(r("1.0.0-rc.5")).unwrap(),
                json!({
                    "version": "1.0.0-rc.5",
                    "prerelease": true,
                    "publishedAt": "2026-10-28T10:00:00Z",
                    "url": "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/Grace-win32-x64-1.0.0-rc.5.zip",
                    "size": 218_000_000,
                    "sha256": "ab".repeat(32),
                })
            );
        }
    }

    mod channels {
        use super::*;

        #[test]
        fn stable_channel_hides_pre_releases() {
            assert_eq!(latest_on(&rs(), Channel::Stable), None);
            assert_eq!(
                latest_on(&rs(), Channel::Prerelease)
                    .map(|x| x.version.to_string())
                    .as_deref(),
                Some("1.0.0-rc.6")
            );
        }

        #[test]
        fn pre_release_channel_also_holds_the_stable_releases() {
            let all = [r("1.0.0"), r("1.0.0-rc.6")];
            assert_eq!(versions(on_channel(&all, Channel::Stable)), ["1.0.0"]);
            assert_eq!(versions(on_channel(&all, Channel::Prerelease)), ["1.0.0", "1.0.0-rc.6"]);
            let flipped = [r("1.0.0-rc.6"), r("1.0.0")];
            assert_eq!(
                latest_on(&flipped, Channel::Prerelease)
                    .map(|x| x.version.to_string())
                    .as_deref(),
                Some("1.0.0")
            );
        }

        #[test]
        fn has_no_latest_in_an_empty_list() {
            assert_eq!(latest_on(&[], Channel::Prerelease), None);
        }
    }

    mod default_update_settings {
        use super::*;

        #[test]
        fn follows_the_running_versions_channel_until_the_person_picks_one_r1_3() {
            assert_eq!(
                channel_of(&UpdateSettings::default(), &v("1.0.0-rc.5")),
                Channel::Prerelease
            );
            assert_eq!(channel_of(&UpdateSettings::default(), &v("1.0.0")), Channel::Stable);
            let picked = UpdateSettings {
                channel: Some(Channel::Stable),
                ..UpdateSettings::default()
            };
            assert_eq!(channel_of(&picked, &v("1.0.0-rc.5")), Channel::Stable);
        }

        #[test]
        fn prompts_on_the_running_versions_channel_when_none_is_picked() {
            let s = UpdateSettings::default();
            let offered =
                |running: &str| prompt_for(&[r("1.0.1-rc.1")], &s, &v(running)).map(|x| x.version.to_string());
            assert_eq!(offered("1.0.0-rc.5").as_deref(), Some("1.0.1-rc.1"));
            assert_eq!(offered("1.0.0"), None);
        }

        #[test]
        fn checks_at_startup_never_updates_by_itself_remembers_nothing_r12() {
            assert_eq!(
                serde_json::to_value(UpdateSettings::default()).unwrap(),
                json!({
                    "channel": null,
                    "checkAtStartup": true,
                    "autoUpdate": false,
                    "skipped": null,
                    "failed": null,
                    "lastChecked": null,
                })
            );
        }
    }

    mod prompt_for {
        use super::*;

        fn offered(rs: &[Release], s: &UpdateSettings, running: &str) -> Option<String> {
            prompt_for(rs, s, &v(running)).map(|x| x.version.to_string())
        }

        #[test]
        fn prompts_once_per_version_above_the_skipped_one_r4_1() {
            let s = UpdateSettings {
                skipped: Some(v("1.0.0-rc.6")),
                ..UpdateSettings::default()
            };
            assert_eq!(offered(&[r("1.0.0-rc.6")], &s, "1.0.0-rc.5"), None);
            assert_eq!(
                offered(&[r("1.0.0-rc.7"), r("1.0.0-rc.6")], &s, "1.0.0-rc.5").as_deref(),
                Some("1.0.0-rc.7")
            );
            let auto = UpdateSettings { auto_update: true, ..s };
            assert_eq!(offered(&[r("1.0.0-rc.7")], &auto, "1.0.0-rc.5"), None);
        }

        #[test]
        fn prompts_for_latest_when_nothing_is_skipped_and_not_when_up_to_date() {
            let s = UpdateSettings::default();
            assert_eq!(
                offered(&[r("1.0.0-rc.6")], &s, "1.0.0-rc.5").as_deref(),
                Some("1.0.0-rc.6")
            );
            assert_eq!(offered(&[r("1.0.0-rc.5")], &s, "1.0.0-rc.5"), None);
            assert_eq!(offered(&[r("1.0.0-rc.4")], &s, "1.0.0-rc.5"), None);
            assert_eq!(offered(&[], &s, "1.0.0-rc.5"), None);
        }

        #[test]
        fn only_looks_at_the_chosen_channel() {
            let s = UpdateSettings {
                channel: Some(Channel::Stable),
                ..UpdateSettings::default()
            };
            assert_eq!(offered(&[r("1.0.1-rc.1")], &s, "1.0.0"), None);
        }
    }

    mod auto_update_to {
        use super::*;

        fn installs(rs: &[Release], s: &UpdateSettings, running: &str) -> Option<String> {
            auto_update_to(rs, s, &v(running)).map(|x| x.version.to_string())
        }

        #[test]
        fn ignores_skipped_but_never_installs_failed_r5_2() {
            let s = UpdateSettings {
                auto_update: true,
                skipped: Some(v("1.0.0-rc.6")),
                failed: Some(v("1.0.0-rc.7")),
                ..UpdateSettings::default()
            };
            assert_eq!(
                installs(&[r("1.0.0-rc.6")], &s, "1.0.0-rc.5").as_deref(),
                Some("1.0.0-rc.6")
            );
            assert_eq!(installs(&[r("1.0.0-rc.7"), r("1.0.0-rc.6")], &s, "1.0.0-rc.5"), None);
        }

        #[test]
        fn does_nothing_when_auto_update_is_off_or_grace_is_up_to_date() {
            let s = UpdateSettings::default();
            assert_eq!(installs(&[r("1.0.0-rc.6")], &s, "1.0.0-rc.5"), None);
            let on = UpdateSettings { auto_update: true, ..s };
            assert_eq!(installs(&[r("1.0.0-rc.5")], &on, "1.0.0-rc.5"), None);
        }
    }

    /// Not in the TypeScript: install progress is written as the window reads it, a step or an error.
    #[test]
    fn update_progress_is_a_step_or_an_error() {
        let step = UpdateProgress::Step {
            version: "1.0.0-rc.5".into(),
            rollback: false,
            step: UpdateStep::Download,
            received: 5,
            total: 10,
        };
        let error = UpdateProgress::Error {
            version: "1.0.0-rc.5".into(),
            rollback: true,
            error: "no space".into(),
        };
        for p in [step, error] {
            let json = serde_json::to_value(&p).unwrap();
            assert_eq!(serde_json::from_value::<UpdateProgress>(json).unwrap(), p);
        }
        assert_eq!(serde_json::to_value(UpdateStep::Restart).unwrap(), json!("restart"),);
    }
}
