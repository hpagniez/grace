//! `src/shared/http.ts`: the HTTP call model and its pure helpers (docs/specs/2026-10-25-http-inspector.md).
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

/// The `n`th of `of` attempts of a retried call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub n: u32,
    pub of: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpCall {
    pub method: String,
    /// None when the step log held no request.
    pub url: Option<String>,
    pub request_headers: Vec<HttpHeader>,
    pub request_body: Option<String>,
    /// None for a call with no response.
    pub status: Option<i32>,
    pub response_headers: Vec<HttpHeader>,
    pub response_body: Option<String>,
    #[serde(with = "crate::model::js_num_opt")]
    pub millis: Option<f64>,
    pub error: Option<String>,
    /// Where a called feature made the call, like `tests/common/create-user.feature:8`.
    pub via: Option<String>,
    #[serde(with = "crate::model::js_num")]
    pub start_time: f64,
    pub attempt: Option<Attempt>,
}
