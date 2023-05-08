
use kernel::domain;
use rocket_contrib::json::Json;
use rocket::response::{Response, Responder};
use rocket::http::Status;
use rocket::request::Request;

use log::warn;

#[derive(Clone, Debug, thiserror::Error, Responder)]
pub(crate) enum Error{
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

#[derive(Debug)]
pub struct ApiError (anyhow::Error);
impl From<anyhow::Error> for ApiError {
    fn from(i: anyhow::Error) -> Self {
        ApiError(i)
    }
}

impl<'r> Responder<'r> for ApiError {
    fn respond_to(self, req: &Request) -> ::std::result::Result<Response<'r>, Status> {
        use diesel::result::Error as DieselError;
        use domain::error::Error as TbError;

        let mut any = self.0;

        warn!("{:?}", any);

        let mut build = Response::build();

        if let Some(err) = any.root_cause().downcast_ref::<TbError>() {
            any = match err {
                TbError::Forbidden(x) => Error::Forbidden(x.into()),
                TbError::NotFound(x) => Error::NotFound(x.into()),
                TbError::BadRequest(x) => Error::BadRequest(x.into()),
                TbError::Conflict(x) => Error::Conflict(x.into()),
                TbError::TryAgain(x) => Error::TryAgain(x),
            }.into();
        }
        
        match any.root_cause().downcast_ref::<DieselError>() {
            Some(DieselError::NotFound) => { 
                    // warn!("{}", any);
                    any = Error::NotFound("Object not found".into()).into();
                },
            Some(DieselError::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation,_)) 
                    => {build.status(Status::BadRequest);},
            _ => {build.status(Status::InternalServerError);}
        }
        if let Some(err) = any.root_cause().downcast_ref::<Error>() {
            build.merge(err.clone().respond_to(req)?);
        }
        // create a standard Body
        // build.header(ContentType::JSON).sized_body(Cursor::new(body));
        
        // Respond. The `Ok` here is a bit of a misnomer. It means we
        // successfully created an error response
        Ok(build.finalize())
    }
}

pub type ApiResult<T> = anyhow::Result<Json<T>>;