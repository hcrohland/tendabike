
pub use tb_domain::schema;
mod async_diesel;

pub use async_diesel::*;

mod store;
mod stravastore;

fn map_to_tb (err: diesel::result::Error) -> tb_domain::Error {
    use diesel::result::Error;
    match err {
        Error::NotFound => tb_domain::Error::NotFound(err.to_string()),
        _ => tb_domain::Error::DatabaseFailure(err.into())
    }
}
