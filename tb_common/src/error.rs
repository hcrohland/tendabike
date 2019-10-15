
use rocket::request::Request;
use rocket::response::{Response, Responder};
use std::io::Cursor;
use rocket::http::{Status, ContentType};


error_chain!{
    types {
        Error, ErrorKind, ResultExt;
    }

    errors{
        Authorize (r: &'static str) {
            description("You need to authorize")
            display("No authorization due to {}", r)
        }
        Forbidden(x: String) {
            description("Forbidden request")
            display("Forbidden request: {}", x)
        }
        NotFound(x: String) {
            description("Object not found")
            display("Could not find object: {}", x)
        }
        BadRequest(x: String) {
            description("Bad request")
            display("Bad Request: {}", x)
        }
        Conflict(x: String) {
            description("Conflict")
            display("Conflict: {}", x)
        }
    }

    foreign_links {
        DbError(diesel::result::Error);
    }
}

// Implement `Responder` for `error_chain`'s `Error` type
// that we just generated
impl<'r> Responder<'r> for Error {
    fn respond_to(self, _: &Request) -> ::std::result::Result<Response<'r>, Status> {
        // Render the whole error chain to a single string
        let start = format!("Error: {}", self);
        let rslt = self.iter().skip(1).fold(start.clone(), |acc, ce| acc + &format!(",\n\tcaused by: {}", ce));

        // Create JSON response
        let resp = json!({
            "status": "failure",
            "message": start,
        }).to_string();

        error!("{}", rslt);
        let status = match self {
                Error(ErrorKind::DbError(error),_) => {
                    use diesel::result::Error as DieselError;
                    match error  {
                        DieselError::NotFound => Status::NotFound,
                        DieselError::DatabaseError(diesel::result::DatabaseErrorKind::ForeignKeyViolation,_) => Status::BadRequest,
                        _ => Status::InternalServerError
                    }
                },
                Error(ErrorKind::NotFound(_),_) => Status::NotFound,
                Error(ErrorKind::Authorize(_),_)  => Status::Unauthorized,
                Error(ErrorKind::Forbidden(_),_) => Status::Forbidden,
                Error(ErrorKind::BadRequest(_),_) => Status::BadRequest,
                Error(ErrorKind::Conflict(_),_) => Status::Conflict,
                _ => Status::InternalServerError,
            };

        // Respond. The `Ok` here is a bit of a misnomer. It means we
        // successfully created an error response
        Ok(Response::build()
            .status(status)
            .header(ContentType::JSON)
            .sized_body(Cursor::new(resp))
            .finalize())
    }
}

pub type TbResult<T> = Result<T, Error>;