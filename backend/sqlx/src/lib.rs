pub use async_diesel::*;

mod async_diesel;
mod store;
mod stravastore;

fn into_domain(err: diesel::result::Error) -> tb_domain::Error {
    use diesel::result::Error;
    match err {
        Error::NotFound => tb_domain::Error::NotFound(err.to_string()),
        _ => tb_domain::Error::AnyFailure(err.into()),
    }
}

fn vec_into<F, T>(from: Vec<F>) -> Vec<T>
where
    T: From<F>,
{
    from.into_iter().map(Into::into).collect()
}

fn option_into<F, T>(from: Option<F>) -> Option<T>
where
    T: From<F>,
{
    from.map(Into::into)
}
