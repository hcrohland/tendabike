
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

    // Initialize server
    rocket::ignite()
       // add config object
        .manage(Config::default())
        // add database pool
        .attach(AppDbConn::fairing())

        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        .mount("/greet", greetings::routes())
        .mount("/part", part::routes())

        // start the server
        .launch();
}