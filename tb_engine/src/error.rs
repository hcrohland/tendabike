//needed for impl Responder
use rocket::request::Request;
use rocket::response::{Response, Responder};
use std::io::Cursor;
use rocket::http::{Status, ContentType};


#[derive(Debug, Error)]
pub enum Error{
    #[error("Forbidden request: {0}")]
    Forbidden(String),
    #[error("Could not find object")]
    NotFound(String),
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("{0}")]
    Any(anyhow::Error)
}

impl From<anyhow::Error> for Error {
    fn from(any: anyhow::Error) -> Self {
        // Do not wrap our errors in Any()
        any.downcast::<Error>().unwrap_or_else(Error::Any)
    }
}

use rocket_contrib::json::Json;
pub type TbResult<T> = anyhow::Result<T>;
pub(crate) type ApiResult<T> = Result<Json<T>, Error>;
pub(crate) fn tbapi<T> (tb: TbResult<T>) -> ApiResult<T> {
    tb.map(Json).map_err(Error::from)
}

// Implement `Responder` for `ApiResult` type
// that we just defined
impl<'r> Responder<'r> for Error {
    fn respond_to(self, _: &Request) -> ::std::result::Result<Response<'r>, Status> {
        // Render the whole error chain to a single string
        let mut rslt = format!("Error: {}", self);

        let status = match self {
                Error::NotFound(x) => { warn!("{}", x); Status::NotFound},
                Error::Forbidden(_) => Status::Forbidden,
                Error::BadRequest(_) => Status::BadRequest,
                Error::Conflict(_) => Status::Conflict,
                Error::Any(err) => {
                    use diesel::result::Error as DieselError;
                    
                    match err.root_cause().downcast_ref::<DieselError>()  {
                        Some(DieselError::NotFound) => { 
                                warn!("{}", err);
                                rslt = format!("Error: {}", Error::NotFound("".into())); 
                                Status::NotFound 
                            },
                        Some(DieselError::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation,_)) => Status::BadRequest,
                        _ => {  Status::InternalServerError}
                    }
                }
                // _ => Status::InternalServerError,
            };

        // Create JSON response
        let resp = json!({
            "status": "failure",
            "message": &rslt,
        }).to_string();

        // Respond. The `Ok` here is a bit of a misnomer. It means we
        // successfully created an error response
        Ok(Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(Cursor::new(resp))
        .finalize())
    }
}