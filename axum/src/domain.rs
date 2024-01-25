use axum::Router;

use crate::appstate::AppState;

mod activity;
mod attachment;
mod part;
mod types;
mod user;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .nest("/user", user::router())
        .nest("/types", types::router())
        .nest("/part", part::router())
        .nest("/part", attachment::router())
        .nest("/activ", activity::router())
}
