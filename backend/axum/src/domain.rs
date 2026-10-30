use axum::Router;
use tb_domain::IdKeyedMap;

use crate::appstate::AppState;

mod activity;
mod attachment;
mod notification;
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

/// The handler's outcome mismatch guard: this route sent its own read
/// (spec #558 §3.1) or write (spec §6.2); any other outcome is a bug, not
/// a domain outcome — the `AnyFailure` names the op (`what`) that lost its
/// payload (`payload`).
pub(crate) fn result_mismatch(what: &str, payload: &str) -> crate::error::AppError {
    crate::error::AppError::TbError(tb_domain::Error::AnyFailure(anyhow::anyhow!(
        "the {what} outcome did not carry its {payload}"
    )))
}

pub(super) fn router() -> Router<AppState> {
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
        .nest("/notification", notification::router())
}
