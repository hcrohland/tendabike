use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use self::schema::{gears, gear_types};
use crate::*;
use crate::user::Person;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};


/// The list of gear types
/// Includes the list of gear types which can be attached to it as parts
/// multiple parts are possible
#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[table_name = "gear_types"]
struct GearTypes {
    /// The primary key
    pub id: i32,
    /// The name
    pub name: String,
    /// Gear types that can be attached
    pub parts: Vec<i32>,
    /// is it a main gear?
    pub main: bool,
}

/// The database's representation of a gear. 
#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[primary_key(id)]
#[table_name = "gears"]
pub struct Gear {
    /// The primary key
    pub id: i32,
    /// The owner
    pub user_id: i32,
    /// The type of the gear
    pub what: i32,
    /// This name of the gear.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
    /// purchase date
    pub purchase: DateTime<Utc>,
    /// usage time
   	time: i32,
    /// Usage distance
	distance: i32,
	/// Overall climbing
    climb: i32,
    /// Overall descending
	descend: i32,
    /// Is the gear attached to an assembly?
    attached_to: Option<i32>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "gears"]
pub struct NewGear {
    pub user_id: i32,
    pub name: String,
    pub vendor: String,
    pub model: String
}

#[derive(AsChangeset, Debug, Clone)]
#[table_name = "gears"]
pub struct UpdateGear {
    pub id: i32,
    pub user_id: i32,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>
}


impl Gear {
    fn gear_by_user (conn: &diesel::PgConnection, uid: i32, main: bool) -> Vec<Gear>{
      //  use crate::schema::gears::dsl::*;

        let main_types = gear_types::table
            .select(gear_types::id)
            .filter(gear_types::main.eq(main))
            .load::<i32>(conn)
            .expect("Error loading gear types");

        gears::table
            .filter(gears::user_id.eq(uid))
            .filter(gears::what.eq(diesel::pg::expression::dsl::any(main_types)))
            .filter(gears::attached_to.is_null())
            .load::<Gear>(conn)
            .expect("Error loading user's gear")
    }
}

#[get("/mygear")]
fn mygear(conn: AppDbConn, user: user::User) -> Json<Vec<Gear>> {    
    Json(Gear::gear_by_user(&conn, user.get_id(), true))
}

#[get("/myspares")]
fn myspares(conn: AppDbConn, user: user::User) -> Json<Vec<Gear>> {    
    Json(Gear::gear_by_user(&conn, user.get_id(), false))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![mygear, myspares]
}