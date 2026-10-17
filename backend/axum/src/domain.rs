use axum::Router;
use tb_domain::IdKeyedMap;
use tb_exec::TxnSource;

use crate::appstate::AppState;

mod activity;
mod attachment;
mod part;
mod partnote;
mod service;
mod serviceplan;
mod shop;
mod types;
mod user;

/// The single live entity in a `Summary` map a create write returns (spec
/// §6.2: a `201` body is the created entity extracted from the write's
/// `Summary`).
pub(crate) fn created_entity<K, V: Clone>(
    entries: &IdKeyedMap<K, V>,
    what: &str,
) -> Result<V, crate::error::AppError> {
    let mut live = entries.live();
    match live.pop() {
        Some(v) if live.is_empty() => Ok(v),
        _ => Err(crate::error::AppError::TbError(
            tb_domain::Error::AnyFailure(anyhow::anyhow!(
                "expected exactly one {what} in the write summary"
            )),
        )),
    }
}

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .nest("/user", user::router())
        .nest("/types", types::router())
        .nest("/shop", shop::router())
        .nest("/part", part::router())
        .nest("/part", partnote::router())
        .nest("/part", attachment::router())
        .nest("/service", service::router())
        .nest("/plan", serviceplan::router())
        .nest("/activ", activity::router())
}
