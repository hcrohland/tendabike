#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate dotenv;


#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

fn main() {

// read environment variables from file
    dotenv::dotenv().ok();

    rocket::ignite().mount("/", routes![index]).launch();
}