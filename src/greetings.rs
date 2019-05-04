use rocket::State;
use rocket_contrib::json::Json;

use super::{
    Config,
    db,
    user::*,
    AppDbConn,
};

#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

#[get("/config")]
fn index_conf(conf: State<Config>, user: User) -> String {
    format!( "{}, user id {}\n", conf.greeting, user.get_id())
}

#[get("/db")]
fn index_db(conn: AppDbConn) -> String {
    db::get_greetings(&conn).join("\n")
}

#[derive(Serialize)]
struct Greeting {
    greetings: Vec<String>,
    user_id: i32,
}

#[get("/json")]
fn index_json(conn: AppDbConn, user: User) -> Json<Greeting> {
    Json( Greeting {
        greetings: db::get_greetings(&conn),
        user_id: user.get_id(),
    })
}
#[get("/exit")]
fn server_exit(admin: Admin) {
    info!( "user id {} requested shutdown\n", admin.get_id());
    std::process::exit(0);
}

pub fn routes () -> Vec<rocket::Route> {
    routes![index, index_conf, index_db, index_json, server_exit]
}