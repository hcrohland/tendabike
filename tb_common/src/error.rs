
use rocket::request::Request;
use rocket::response::{Response, Responder};
use std::io::Cursor;
use rocket::http::{Status, ContentType};
use rocket_contrib::json::Json;

#[derive(Debug, Error, Responder)]
pub enum Error{
    #[response(status = 403)]
    #[error("Forbidden request: {0}")]
    Forbidden(String),
    #[response(status = 404)]
    #[error("Object not found")]
    NotFound(String),
    #[response(status = 400)]
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[response(status = 409)]
    #[error("Conflict: {0}")]
    Conflict(String),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ApiError (#[from] anyhow::Error);

macro_rules! respond_for {
    ($rb:ident, $any:ident, $req:ident, [$($t:ty),+] ) => {
        $( if $any.is::<$t>() {
            let err = $any.downcast::<$t>().unwrap();
            $rb.merge(err.respond_to($req)?);
        } else )+
        {}
    };
}

impl<'r> Responder<'r> for ApiError {
    fn respond_to(self, req: &Request) -> ::std::result::Result<Response<'r>, Status> {
        use diesel::result::Error as DieselError;
        let mut any = self.0;
        let mut status = Status::InternalServerError;

        warn!("{:?}", any);
        match any.root_cause().downcast_ref::<DieselError>() {
            Some(DieselError::NotFound) => { 
                    // warn!("{}", any);
                    any = Error::NotFound("Object not found".into()).into();
                },
            Some(DieselError::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation,_)) => status = Status::BadRequest,
            _ =>   ()
        }

        // Create JSON response
        let body = json!({
            "status": "failure",
            "message": &format!("Error: {}", any),
        }).to_string();

        let mut build = Response::build();
        // set a default Status
        build.status(status);
        
        respond_for!(build, any, req, [Error]);
        // create a standard Body
        build.header(ContentType::JSON).sized_body(Cursor::new(body));
        
        // Respond. The `Ok` here is a bit of a misnomer. It means we
        // successfully created an error response
        Ok(build.finalize())
    }
}


pub type TbResult<T> = Result<T,anyhow::Error>;
pub type ApiResult<T> = Result<Json<T>,ApiError>;

pub fn tbapi<T> (tb: TbResult<T>) -> ApiResult<T> {
    tb.map(Json).map_err(ApiError::from)
}