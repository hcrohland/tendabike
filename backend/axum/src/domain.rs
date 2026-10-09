use axum::Router;
use tb_exec::TxnSource;
use tb_strava::StravaStore;

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

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
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
