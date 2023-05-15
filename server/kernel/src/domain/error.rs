
use thiserror::Error;
pub use anyhow::{Context, ensure, bail};

#[derive(Clone, Debug, Error)]
pub enum Error{
    #[error("User not authenticated: {0}")]
    NotAuth(String),
    #[error("Forbidden request: {0}")]
    Forbidden(String),
    #[error("Object not found: {0}")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Try again: {0}")]
    TryAgain(&'static str),
}

pub type AnyResult<T> = Result<T,anyhow::Error>;