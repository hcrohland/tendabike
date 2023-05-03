

mod user;
pub use user::Admin;

mod error;
pub use error::*;

pub mod server;

#[database("app_db")]
pub struct AppDbConn(crate::AppConn);

