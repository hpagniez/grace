//! What Electron Grace's `src/shared` holds: the types main and the window shared, and their pure rules. Each
//! submodule is the port of one `src/shared/<name>.ts`, its tests those of `<name>.test.ts`. JSON keys stay as they
//! are on disk and in Electron Grace (camelCase), so the data files read the same.
pub mod env;
pub mod events;
pub mod flaky;
pub mod history;
pub mod http;
pub mod js;
pub mod node;
pub mod status;
pub mod tags;
pub mod theme;
pub mod threads;
pub mod trends;
pub mod updates;
pub mod version;

pub use node::{Layout, NodeKind, Status, TestNode};

/// A JavaScript number as `JSON.stringify` writes it: a whole number without `.0`. Use with
/// `#[serde(with = "crate::model::js_num")]` on an `f64` (times, durations), so the files Grace writes read the same
/// as Electron Grace's.
pub mod js_num {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(n: &f64, s: S) -> Result<S::Ok, S::Error> {
        if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
            s.serialize_i64(*n as i64)
        } else {
            s.serialize_f64(*n)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        f64::deserialize(d)
    }
}

/// `js_num` for an `Option<f64>`: None is `null`.
pub mod js_num_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(n: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
        match n {
            Some(n) => super::js_num::serialize(n, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
        Option::<f64>::deserialize(d)
    }
}

#[cfg(test)]
mod js_num_tests {
    #[derive(serde::Serialize)]
    struct T {
        #[serde(with = "super::js_num")]
        a: f64,
        #[serde(with = "super::js_num_opt")]
        b: Option<f64>,
    }

    #[test]
    fn whole_numbers_have_no_fraction() {
        let s = serde_json::to_string(&T {
            a: 1_700_000_000_000.0,
            b: Some(1.5),
        })
        .unwrap();
        assert_eq!(s, r#"{"a":1700000000000,"b":1.5}"#);
        assert_eq!(
            serde_json::to_string(&T { a: 0.0, b: None }).unwrap(),
            r#"{"a":0,"b":null}"#
        );
    }
}
