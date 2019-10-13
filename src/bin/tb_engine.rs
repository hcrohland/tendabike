
extern crate rocket;
extern crate tb_engine;
use tb_engine::*;
fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket().launch();
}