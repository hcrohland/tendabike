pub use async_diesel::*;

mod async_diesel;
mod store;
mod stravastore;

fn into_domain(err: diesel::result::Error) -> tb_domain::Error {
    use diesel::result::Error;
    match err {
        Error::NotFound => tb_domain::Error::NotFound(err.to_string()),
        _ => tb_domain::Error::DatabaseFailure(err),
    }
}
