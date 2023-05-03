
use rocket::response::Responder;

pub use anyhow::Context;


#[derive(Clone, Debug, Error, Responder)]
pub enum Error{
    #[response(status = 401)]
    #[error("User not authenticated: {0}")]
    NotAuth(String),
    #[response(status = 403)]
    #[error("Forbidden request: {0}")]
    Forbidden(String),
    #[response(status = 404)]
    #[error("Object not found: {0}")]
    NotFound(String),
    #[response(status = 400)]
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[response(status = 409)]
    #[error("Conflict: {0}")]
    Conflict(String),
    #[response(status = 429)]
    #[error("Try again: {0}")]
    TryAgain(&'static str),
}

pub type TbResult<T> = Result<T,anyhow::Error>;
