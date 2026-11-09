//! `src/shared/version.ts`: version rules for in-app updates (docs/specs/2026-10-28-app-updates.md R1).
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// `X.Y.Z` or `X.Y.Z-rc.N`, the only forms ADR 0015 allows.
///
/// Differs from the TypeScript, where a version is a string until it is parsed, and every comparison parses again and
/// may throw: a `Version` is valid by construction, orders with `Ord` (a release above any of its release candidates,
/// SemVer precedence) and is written to and read from JSON as its text. The parts are `u64`, where JavaScript's are
/// doubles: a number past `u64`, twenty digits, is no version here and was a (lossy) one there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// The `N` of `-rc.N`; `None` for a release.
    pub rc: Option<u64>,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64, rc: Option<u64>) -> Self {
        Version {
            major,
            minor,
            patch,
            rc,
        }
    }

    /// True for `X.Y.Z-rc.N` (the TypeScript's `isPrerelease`, of a version that is already valid).
    pub fn is_prerelease(&self) -> bool {
        self.rc.is_some()
    }
}

/// SemVer precedence: a release ranks above any of its release candidates.
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let rc = |v: &Version| v.rc.map_or((1, 0), |n| (0, n));
        (self.major, self.minor, self.patch, rc(self)).cmp(&(other.major, other.minor, other.patch, rc(other)))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        match self.rc {
            Some(n) => write!(f, "-rc.{n}"),
            None => Ok(()),
        }
    }
}

/// The text that is no version. Its `Display` is the TypeScript's error message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidVersion(pub String);

impl fmt::Display for InvalidVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid version: {}", self.0)
    }
}

impl std::error::Error for InvalidVersion {}

/// A number of a version: `0`, or digits with no leading zero (`(0|[1-9]\d*)`, with `\d` ASCII only).
fn number(s: &str) -> Option<u64> {
    let digits = s.bytes().all(|b| b.is_ascii_digit());
    (digits && (s == "0" || !s.starts_with('0')))
        .then(|| s.parse().ok())
        .flatten()
}

impl FromStr for Version {
    type Err = InvalidVersion;

    /// The TypeScript's `/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/`, written as its parts.
    fn from_str(s: &str) -> Result<Self, InvalidVersion> {
        let invalid = || InvalidVersion(s.to_string());
        let (core, rc) = match s.split_once("-rc.") {
            Some((core, rc)) => (core, Some(number(rc).ok_or_else(invalid)?)),
            None => (s, None),
        };
        let mut parts = core.split('.');
        let mut next = || parts.next().and_then(number);
        let (major, minor, patch) = (next(), next(), next());
        match (major, minor, patch, parts.next()) {
            (Some(major), Some(minor), Some(patch), None) => Ok(Version {
                major,
                minor,
                patch,
                rc,
            }),
            _ => Err(invalid()),
        }
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}

/// Parses `X.Y.Z` or `X.Y.Z-rc.N`, the only forms ADR 0015 allows. Anything else is `None`.
pub fn parse_version(s: &str) -> Option<Version> {
    s.parse().ok()
}

/// `parse_version` of untrusted JSON: anything that is not a string is `None` as well.
///
/// The TypeScript's `typeof s === "string"` guard inside `parseVersion`; a `&str` cannot be anything else, so it lives
/// here, where a value of unknown type comes in.
pub fn parse_version_value(x: &Value) -> Option<Version> {
    x.as_str().and_then(parse_version)
}

/// SemVer precedence of two version texts. Fails when either side is not a valid version (the TypeScript throws).
pub fn compare_versions(a: &str, b: &str) -> Result<Ordering, InvalidVersion> {
    match (a.parse::<Version>(), b.parse::<Version>()) {
        (Ok(x), Ok(y)) => Ok(x.cmp(&y)),
        (Ok(_), Err(e)) | (Err(e), _) => Err(e),
    }
}

/// True for a valid `X.Y.Z-rc.N`.
pub fn is_prerelease(v: &str) -> bool {
    parse_version(v).is_some_and(|v| v.is_prerelease())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod parse_version {
        use super::*;

        #[test]
        fn parses_only_x_y_z_and_x_y_z_rc_n() {
            assert_eq!(
                parse_version("1.0.0-rc.4"),
                Some(Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                    rc: Some(4)
                })
            );
            assert_eq!(
                parse_version("1.2.3"),
                Some(Version {
                    major: 1,
                    minor: 2,
                    patch: 3,
                    rc: None
                })
            );
            for bad in [
                "v1.0.0",
                "1.0",
                "1.0.0-rc.04",
                "01.0.0",
                "1.0.0-rc.4.1",
                "1.0.0-beta.1",
                "1.0.0-rc",
                "",
                "1.0.0\n",
            ] {
                assert_eq!(parse_version(bad), None, "{bad:?}");
            }
        }

        #[test]
        fn refuses_anything_that_is_not_a_string() {
            // the TypeScript's `["1.0.0"], 1, null, undefined`; `undefined` has no JSON value
            for bad in [json!(["1.0.0"]), json!(1), Value::Null] {
                assert_eq!(parse_version_value(&bad), None, "{bad}");
            }
            assert_eq!(parse_version_value(&json!("1.2.3")), Some(Version::new(1, 2, 3, None)));
        }

        /// Not in the TypeScript: the rest of what the pattern says, and `Display` and JSON.
        #[test]
        fn is_the_pattern_and_writes_itself_back() {
            for bad in [
                "1.0.0-rc.",
                "1..0",
                ".1.0",
                "1.0.0.",
                "1.0.0-rc.4-rc.5",
                "+1.0.0",
                "1.0.0 ",
                "١.0.0",
                "1.0.-1",
            ] {
                assert_eq!(parse_version(bad), None, "{bad:?}");
            }
            for ok in ["0.0.0", "10.20.30", "1.0.0-rc.0", "1.0.0-rc.10"] {
                let v = parse_version(ok).unwrap();
                assert_eq!(v.to_string(), ok);
                assert_eq!(serde_json::to_value(v).unwrap(), json!(ok));
                assert_eq!(serde_json::from_value::<Version>(json!(ok)).unwrap(), v);
            }
            assert!(serde_json::from_value::<Version>(json!("v1.0.0")).is_err());
        }
    }

    mod compare_versions {
        use super::*;

        #[test]
        fn orders_by_semver_precedence() {
            let mut sorted = vec!["1.0.0", "1.0.0-rc.10", "0.9.9", "1.0.0-rc.9", "1.0.1-rc.1"];
            sorted.sort_by(|a, b| compare_versions(a, b).unwrap());
            assert_eq!(sorted, ["0.9.9", "1.0.0-rc.9", "1.0.0-rc.10", "1.0.0", "1.0.1-rc.1"]);
        }

        #[test]
        fn is_0_for_equal_versions_release_candidates_or_not() {
            assert_eq!(compare_versions("1.0.0", "1.0.0"), Ok(Ordering::Equal));
            assert_eq!(compare_versions("1.0.0-rc.4", "1.0.0-rc.4"), Ok(Ordering::Equal));
        }

        #[test]
        fn throws_when_either_side_is_invalid() {
            assert_eq!(
                compare_versions("v1.0.0", "1.0.0"),
                Err(InvalidVersion("v1.0.0".into()))
            );
            assert_eq!(compare_versions("1.0.0", "x"), Err(InvalidVersion("x".into())));
            // the message names the first invalid side, `b` only when `a` is valid
            assert_eq!(
                compare_versions("v1", "x").unwrap_err().to_string(),
                "Invalid version: v1"
            );
            assert_eq!(
                compare_versions("1.0.0", "x").unwrap_err().to_string(),
                "Invalid version: x"
            );
        }
    }

    mod is_prerelease {
        use super::*;

        #[test]
        fn is_true_only_for_a_valid_release_candidate() {
            assert!(is_prerelease("1.0.0-rc.0"));
            assert!(!is_prerelease("1.0.0"));
            assert!(!is_prerelease("1.0.0-beta.1"));
        }
    }
}
