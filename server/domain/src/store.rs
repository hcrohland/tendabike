use s_diesel::AppConn;

use crate::traits::Store;

mod types;
mod part;
mod user;
mod activity;
mod attachment;

impl Store for AppConn {
}