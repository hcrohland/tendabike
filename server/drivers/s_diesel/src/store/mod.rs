use crate::AppConn;

use domain::Store;

mod types;
mod part;
mod user;
mod activity;
mod attachment;

impl Store for AppConn {
}