use super::*;
use rocket_contrib::json::Json;
use rocket::response::status;

mod activity;
pub(super) use activity::routes as activity;
mod user;
pub(super) use user::routes as user;
mod types;
pub(super) use types::routes as types;
mod part;
pub(super) use part::routes as part;
mod attachment;
pub(super) use attachment::routes as attachment;
