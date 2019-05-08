use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use self::schema::{parts, part_types};
use crate::user::*;
use crate::*;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};


/// The list of part types
/// Includes the list of part types which can be attached to it as parts
/// multiple parts are possible
#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[table_name = "part_types"]
struct PartTypes {
    /// The primary key
    pub id: i32,
    /// The name
    pub name: String,
    /// Part types that can be attached
    pub parts: Vec<i32>,
    /// is it a main part?
    pub main: bool,
}

impl PartTypes {
    fn all (conn: &AppConn) -> Vec<PartTypes> {
        part_types::table.load::<PartTypes>(conn).expect("error loading PartTypes")
    }
}

/// The database's representation of a part. 
#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations)]
#[belongs_to(PartTypes, foreign_key = "what")]
#[primary_key(id)]
#[table_name = "parts"]
pub struct Part {
    /// The primary key
    pub id: i32,
    /// The owner
    pub user_id: i32,
    /// The type of the part
    pub what: i32,
    /// This name of the part.
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
    /// Is the part attached to an assembly?
    attached_to: Option<i32>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "parts"]
pub struct NewPart {
    pub user_id: i32,
    pub name: String,
    pub vendor: String,
    pub model: String
}

#[derive(AsChangeset, Debug, Clone)]
#[table_name = "parts"]
pub struct UpdatePart {
    pub id: i32,
    pub user_id: i32,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>
}

#[derive(Serialize)]
pub struct Assembly {
    pub part: Part,
    pub subs: Box<[Assembly]>,
}

impl Assembly {
    pub fn new (part: Part, conn: &AppConn) -> Assembly {
        use crate::schema::parts::dsl::*;
        

        let subs = parts
            .filter(attached_to.eq(part.id))
            .load::<Part>(conn).expect("Error loading subparts");
    
        let subs = subs.into_iter()
            .map(|x: Part| -> Assembly {
                    Assembly::new(x, conn)
            });

        let subs: Vec<Assembly> = subs.collect();
        let subs = subs.into_boxed_slice();

        Assembly {
            part: part, 
            subs: subs,
        }
    }

}


impl Part {
    fn get (part: i32, _owner: User, conn: &AppConn) -> Part {
        use crate::schema::parts::dsl::*;

        parts.find(part).first(conn).expect("error loading part")
    }

    fn part_by_user (conn: &AppConn, uid: i32, main: bool) -> Vec<Part>{
      //  use crate::schema::parts::dsl::*;

        let types = part_types::table
            .filter(part_types::main.eq(main))
            .load::<PartTypes>(conn)
            .expect("Error loading types");

        Part::belonging_to(&types)
            .filter(parts::user_id.eq(uid))
            .filter(parts::attached_to.is_null())
            .load::<Part>(conn)
            .expect("Error loading user's part")
    }

    fn subparts (part: i32, conn: &AppConn) -> Box<[Part]> {
        use crate::schema::parts::dsl::*;

        parts.filter(attached_to.eq(part)).load(conn).expect("Error loading subparts").into_boxed_slice()
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(PartTypes::all(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> Json<Part> {
    Json(Part::get(part, user, &conn))
}

#[get("/ass/<part>")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> Json<Assembly> {
    let part = Part::get(part, user, &conn);
    Json(Assembly::new(part, &conn))
}

#[get("/mygear")]
fn mygear(user: User, conn: AppDbConn) -> Json<Vec<Part>> {    
    Json(Part::part_by_user(&conn, user.get_id(), true))
}

#[get("/myspares")]
fn myspares(user: User, conn: AppDbConn) -> Json<Vec<Part>> {    
    Json(Part::part_by_user(&conn, user.get_id(), false))
}

#[get("/subpart/<part>")]
fn subparts(part: i32, _user: User,conn: AppDbConn) -> Json<Box<[Part]>> {    
    Json(Part::subparts(part, &conn))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, get_assembly, mygear, myspares, subparts]
}