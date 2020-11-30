extern crate rocket;
extern crate tendabike;
use tendabike::*;
fn main() {
    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket()
    .attach(rocket::fairing::AdHoc::on_launch("Launch Message", |rocket| {
        let c = rocket.config();
        eprintln!("\nInfo: TendaBike running on {}:{}\n", c.address, c.port);
    })).launch();
}
