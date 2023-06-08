use crate::AppConn;

use domain::traits::Store;

mod types;
mod part;
mod user;
mod activity;
mod attachment;

impl Store for AppConn {
}