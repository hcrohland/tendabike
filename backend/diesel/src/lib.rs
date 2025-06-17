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

fn vec_into<F, T>(from: Result<Vec<F>, diesel::result::Error>) -> tb_domain::TbResult<Vec<T>>
where
    T: From<F>,
{
    from.map_err(into_domain)
        .map(|i| i.into_iter().map(Into::into).collect())
}
