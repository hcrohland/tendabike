use std::collections::HashMap;

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

/// List of of all valid part types.
/// 
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
#[table_name = "part_types"]
pub struct PartTypes {
    /// The primary key
    pub id: i32,
    /// The display name
    pub name: String,
    /// Part types that can be attached
    pub hooks: Vec<i32>,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: bool,
}

/// The database's representation of a part. 
#[derive(Clone, Debug, PartialEq, 
        Serialize, Deserialize, 
        Queryable, Identifiable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "parts"]
#[belongs_to(PartTypes, foreign_key = "what")]
#[belongs_to(Part, foreign_key = "attached_to")]
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
   	pub time: i32,
    /// Usage distance
	pub distance: i32,
	/// Overall climbing
    pub climb: i32,
    /// Overall descending
	pub descend: i32,
    /// Is the part attached to an assembly?
    pub attached_to: Option<i32>,
    /// usage count
    pub count: i32,
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

//#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub type Assembly = HashMap<i32, Part>;

pub trait ATrait {
    fn part (&self, part: i32) -> Option<&Part>;
}

impl ATrait for Assembly {
    fn part (&self, part: i32) -> Option<&Part> {
        self.get(&part)
    }
}

impl Part {
    fn types (conn: &AppConn) -> Vec<PartTypes> {
        part_types::table.order_by(part_types::id).load::<PartTypes>(conn).expect("error loading PartTypes")
    }

    fn get (part: i32, _owner: &Person, conn: &AppConn) -> TbResult<Part> {
        Ok(parts::table.find(part).first(conn)?)
    }

    fn parts_by_user (user: &Person, main: bool, conn: &AppConn) -> TbResult<Vec<Part>>{
        use crate::schema::parts::dsl::*;

        let types = part_types::table
            .filter(part_types::main.eq(main))
            .load::<PartTypes>(conn)?;

        Ok(Part::belonging_to(&types)
            .filter(owner.eq(user.get_id()))
            .filter(attached_to.is_null())
            .order_by(id)
            .load::<Part>(conn)?)
    }

   fn _parts_by_part (&self, conn: &AppConn) -> TbResult<HashMap<i32, Part>>{
        use crate::schema::parts::dsl::*;

        Ok(Part::belonging_to(self).order_by(id)
            .load::<Part>(conn)?.into_iter().map(|x| {(x.id, x)}).collect())
    }

   fn apply (mut self, usage: &Usage, conn: &AppConn) -> TbResult<Part> {
        if let Some(func) = usage.op {
            info!("Applying usage to part {}", self.id);

            func(& mut self.time, usage.time);
            func(& mut self.distance, usage.distance);
            func(& mut self.climb, usage.climb);
            func(& mut self.descend, usage.descend);
            func(& mut self.count, usage.count);

            Ok(self.save_changes::<Part>(conn)?)
        } else {
            Ok(self)
        }
    }

    fn attached_to(&self, _at_time: DateTime<Utc>, conn: &AppConn) -> TbResult<Vec<Part>> {
        Ok(Part::belonging_to(self)
                .order_by(parts::id)  // need this for stable test results
                .for_update()
                .load::<Part>(conn)?)
    }

    fn traverse (self, map: & mut HashMap<i32, Part>, usage: &Usage, conn: &AppConn) -> TbResult<()> {
        self.attached_to(usage.start, conn)?
                .into_iter().map(|x| x.traverse(map, usage, conn))
                .for_each(drop);

        map.insert(self.id, self.apply(usage, conn)?);
        Ok(())
    }

    pub fn utilize (map: & mut HashMap<i32, Part>, usage: Usage, part_id: i32, user: &Person, conn: &AppConn) -> TbResult<()> {
        Part::get(part_id, user, conn)?
                .traverse (map, &usage, conn)
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(Part::types(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Part>> {
    Part::get(part, &user, &conn).map (|x| Json(x))
}

#[get("/<part>?assembly")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    let mut map = HashMap::new();

    Part::utilize(&mut map, Usage::none(), part, &user, &conn)?;
    
    Ok(Json(map))
}

#[get("/mygear")]
fn mygear(user: User, conn: AppDbConn) -> TbResult<Json<Vec<Part>>> {    
    Part::parts_by_user(&user, true, &conn).map(|x| Json(x))
}

#[get("/myspares")]
fn myspares(user: User, conn: AppDbConn) -> TbResult<Json<Vec<Part>>> {    
    Part::parts_by_user(&user, false, &conn).map(|x| Json(x))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, get_assembly, mygear, myspares]
}