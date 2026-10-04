use crate::SqlxConn;
use tb_domain::Store;

mod activity;
mod attachment;
mod part;
mod partnote;
mod service;
mod serviceplan;
mod shop;
mod usage;
mod user;

impl<'c> Store for SqlxConn<'c> {}
