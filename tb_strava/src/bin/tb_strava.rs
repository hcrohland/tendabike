extern crate simplelog;

use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};
use rocket_contrib::templates::Template;

extern crate tb_strava;
use tb_strava::*;

fn init_logging (){
    use std::fs::File;
    const LOGFILE_NAME: & str = "tb_strava.log";
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, simplelog::Config::default(), simplelog::TerminalMode::Stdout).expect("Couldn't get terminal logger"),
        WriteLogger::new(
            LevelFilter::Debug,
            simplelog::Config::default(),
            File::create(LOGFILE_NAME).expect("Couldn't create logfile"),
        ),
    ])
    .expect("Can't get logger.");

}

fn init_environment () {
    dotenv::dotenv().expect("Couldn't read environment");

    init_logging();       
}

pub fn ignite_rocket () -> rocket::Rocket {
    dotenv::dotenv().ok();
    // Initialize server
    rocket::ignite()
        // add config object
        .manage(Config::default())
        // add database pool
        .attach(AppDbConn::fairing())
        // add oauth2 flow
        .attach(auth::fairing())
        // add Template support
        .attach(Template::fairing())
        // redirects catcher
        .register(auth::catchers())
        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        // .mount("/auth", auth::routes())
        .mount("/user", user::routes())
        .mount("/tb", tb::routes())
        // .mount("/activ", activity::routes())
        // .mount("/attach", attachment::routes())
}

fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket().launch();
}

