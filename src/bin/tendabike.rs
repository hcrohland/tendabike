
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

extern crate tendabike;

use tendabike::*;

#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    tendabike::ignite_rocket().launch();
}