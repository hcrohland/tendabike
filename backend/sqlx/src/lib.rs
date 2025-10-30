pub use tb_sqlx::*;

mod store;
mod stravastore;
mod tb_sqlx;

fn into_domain(err: sqlx::Error) -> tb_domain::Error {
    match err {
        sqlx::Error::RowNotFound => tb_domain::Error::NotFound(err.to_string()),
        _ => tb_domain::Error::DatabaseFailure(err.into()),
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
