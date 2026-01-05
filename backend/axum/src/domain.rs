use axum::Router;

use crate::appstate::AppState;

mod activity;
mod attachment;
mod part;
mod service;
mod serviceplan;
mod shop;
mod types;
mod user;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .nest("/user", user::router())
        .nest("/types", types::router())
        .nest("/shop", shop::router())
        .nest("/part", part::router())
        .nest("/part", attachment::router())
        .nest("/service", service::router())
        .nest("/plan", serviceplan::router())
        .nest("/activ", activity::router())
}
