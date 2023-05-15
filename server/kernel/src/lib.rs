#![feature(drain_filter)]
#![warn(clippy::all)]


#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod s_diesel;
use s_diesel::*;
pub mod domain;
pub mod stravatrait;