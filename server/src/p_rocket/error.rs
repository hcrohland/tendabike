
use rocket_contrib::json::Json;
use rocket::response::{Response, Responder};
use rocket::http::Status;
use rocket::request::Request;

use log::warn;
use crate::error::{Error, TbResult};

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

        if let Some(err) = any.root_cause().downcast_ref::<DieselError>() {
            match err {
                    DieselError::NotFound => { 
                            // warn!("{}", any);
                            any = Error::NotFound("Object not found".into()).into();
                        },
                    DieselError::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation,_)
                        => {build.status(Status::BadRequest);},
                    _ 
                        => {build.status(Status::InternalServerError);}
                };
        };

        if let Some(err) = any.root_cause().downcast_ref::<Error>() {
            let status = match err {
                Error::NotAuth(_)      => Status::Unauthorized,
                Error::Forbidden(_)    => Status::Forbidden,
                Error::NotFound(_)     => Status::NotFound,
                Error::BadRequest(_)   => Status::BadRequest,
                Error::Conflict(_)     => Status::Conflict,
                Error::TryAgain(_)     => Status::TooManyRequests,                       
            };
            build
                .status(status)
                .merge(err.clone().respond_to(req)?);
        }
               
        // Respond. The `Ok` here is a bit of a misnomer. It means we
        // successfully created an error response
        Ok(build.finalize())
    }
}

pub type ApiResult<T> = TbResult<Json<T>>;