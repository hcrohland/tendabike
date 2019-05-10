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
    pub hooks: Vec<i32>,
    /// is it a main part?
    pub main: bool,
}

/// The database's representation of a part. 
#[derive(Clone, Debug, Queryable, Serialize, Identifiable, Associations, AsChangeset)]
#[belongs_to(PartTypes, foreign_key = "what")]
#[primary_key(id)]
#[table_name = "parts"]
pub struct Part {
    /// The primary key
    pub id: i32,
    /// The owner
    pub owner: i32,
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
    /// usage count
    count: i32,
}

/*
#[derive(Insertable, Debug, Clone)]
#[table_name = "parts"]
pub struct NewPart {
    pub owner: i32,
    pub name: String,
    pub vendor: String,
    pub model: String
}

#[derive(AsChangeset, Debug, Clone)]
#[table_name = "parts"]
pub struct UpdatePart {
    pub id: i32,
    pub owner: i32,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>
}
*/

#[derive(Serialize)]
pub struct Assembly {
    pub part: Part,
    pub subs: Box<[Assembly]>,
}

impl Part {
    fn types (conn: &AppConn) -> Vec<PartTypes> {
        part_types::table.load::<PartTypes>(conn).expect("error loading PartTypes")
    }

    fn get (part: i32, _owner: &User, conn: &AppConn) -> Option<Part> {
        parts::table.find(part).first(conn).ok()
    }

    fn part_by_user (user: &Person, main: bool, conn: &AppConn) -> Vec<Part>{
        let types = part_types::table
            .filter(part_types::main.eq(main))
            .load::<PartTypes>(conn)
            .expect("Error loading types");

        Part::belonging_to(&types)
            .filter(parts::owner.eq(user.get_id()))
            .filter(parts::attached_to.is_null())
            .load::<Part>(conn)
            .expect("Error loading user's part")
    }

    fn traverse (self, usage: Option<&Usage>, conn: &AppConn) -> Assembly {
        Assembly {
            subs: parts::table
                .filter(parts::attached_to.eq(self.id))
                .load::<Part>(conn).expect("Error loading subparts")
                .into_iter().map(|x| x.traverse(usage, conn)).collect::<Vec<_>>()
                .into_boxed_slice(),
            part: self.apply(usage, conn),
        }
    }

    pub fn register (usage: Option<&Usage>, id: i32, user: &User, conn: &AppConn) -> Option<Assembly> {
        Some(Part::get(id, user, conn)?
                .traverse (usage, conn))
    }

    fn apply (mut self, usage: Option<&Usage>, conn: &AppConn) -> Part {
        if let Some(us) = usage {
            let func = us.op;

            func(& mut self.time, us.time);
            func(& mut self.distance, us.distance);
            func(& mut self.climb, us.climb);
            func(& mut self.descend, us.descend);
            func(& mut self.count, 1);

            self.save_changes::<Part>(conn).expect("error saving part")
        } else {
            self
        }
    }
 }

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(Part::types(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> Option<Json<Part>> {
    Some (Json(Part::get(part, &user, &conn)?))
}

#[get("/<part>?assembly")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> Option<Json<Assembly>> {
    Some(Json(
        Part::get(part, &user, &conn)?
            .traverse(None, &conn)
    ))
}

#[get("/mygear")]
fn mygear(user: User, conn: AppDbConn) -> Json<Vec<Part>> {    
    Json(Part::part_by_user(&user, true, &conn))
}

#[get("/myspares")]
fn myspares(user: User, conn: AppDbConn) -> Json<Vec<Part>> {    
    Json(Part::part_by_user(&user, false, &conn))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, get_assembly, mygear, myspares]
}