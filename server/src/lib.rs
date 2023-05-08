#![feature(drain_filter)]
#![warn(clippy::all)]


#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod drivers;
pub mod domain;

pub use domain::*;

use drivers::persistence::*;
