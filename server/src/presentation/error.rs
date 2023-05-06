
use rocket_contrib::json::Json;
use rocket::response::{Response, Responder};
use rocket::http::Status;
use rocket::request::Request;

use crate::error::*;

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
        let mut any = self.0;

        warn!("{:?}", any);

        let mut build = Response::build();

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

pub type ApiResult<T> = TbResult<Json<T>>;