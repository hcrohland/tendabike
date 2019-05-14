
extern crate rocket;
extern crate tendabike;
use tendabike::*;
fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket().launch();
}