
use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

#[database("app_db")]
pub struct AppDbConn(diesel::PgConnection);

#[derive(Queryable)]
pub struct Greeting {
    pub id: i32,
    pub text: String,
}


pub fn get_greeting(conn: &diesel::PgConnection) -> String {
    use crate::schema::greetings::dsl::*;
    
    let result = greetings
        .select(text)
        .first(conn)
        .expect("Error loading posts");

    result 

}