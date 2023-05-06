pub(crate) use anyhow::Context;

pub mod activity;
pub mod auth;
pub mod gear;

pub use crate::*;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("authorization needed: {0}")]
    Authorize(&'static str),
}

use serde_json::Value as jValue;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct JSummary {
    activities: Vec<jValue>,
    parts: Vec<jValue>,
    attachments: Vec<jValue>
}
