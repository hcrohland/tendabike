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

#[cfg(test)]
mod tests {
    use super::*;
    use tb_domain::{Error, PartId};

    #[test]
    fn into_domain_row_not_found_maps_to_not_found() {
        let err = into_domain(sqlx::Error::RowNotFound);
        assert!(
            matches!(err, Error::NotFound(msg) if msg == "no rows returned by a query that expected to return at least one row")
        );
    }

    #[test]
    fn into_domain_other_errors_map_to_database_failure() {
        let err = into_domain(sqlx::Error::InvalidArgument("boom".to_string()));
        assert!(matches!(err, Error::DatabaseFailure(_)));
    }

    #[test]
    fn vec_into_maps_every_element() {
        let ids: Vec<PartId> = vec_into(vec![1, 2, 3]);
        assert_eq!(ids, vec![PartId::from(1), PartId::from(2), PartId::from(3)]);
        let empty: Vec<PartId> = vec_into(Vec::<i32>::new());
        assert!(empty.is_empty());
    }

    #[test]
    fn option_into_maps_some_and_none() {
        let some: Option<PartId> = option_into(Some(7));
        assert_eq!(some, Some(PartId::from(7)));
        let none: Option<PartId> = option_into(None::<i32>);
        assert_eq!(none, None);
    }
}
