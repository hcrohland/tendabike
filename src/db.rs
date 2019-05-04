
use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

#[derive(Queryable)]
pub struct Greeting {
    pub id: i32,
    pub text: String,
}


pub fn get_greetings(conn: &diesel::PgConnection) -> Vec<String> {
    use crate::schema::greetings::dsl::*;
    
    let result = greetings
        .select(text)
        .load(conn)
        .expect("Error loading posts");

    result 

}