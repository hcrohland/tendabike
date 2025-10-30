use crate::SqlxConn;
use tb_domain::Store;

mod activity;
mod attachment;
mod part;
mod service;
mod serviceplan;
mod usage;
mod user;

impl Store for SqlxConn {}
