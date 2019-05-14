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
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
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
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, AsChangeset, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Assembly {
    pub part: Part,
    pub subs: Box<[Assembly]>,
}

impl Part {
    fn types (conn: &AppConn) -> Vec<PartTypes> {
        part_types::table.order_by(part_types::id).load::<PartTypes>(conn).expect("error loading PartTypes")
    }

    fn get (part: i32, _owner: &Person, conn: &AppConn) -> QueryResult<Part> {
        parts::table.find(part).first(conn)
    }

    fn part_by_user (user: &Person, main: bool, conn: &AppConn) -> QueryResult<Vec<Part>>{
        use crate::schema::parts::dsl::*;

        let types = part_types::table
            .filter(part_types::main.eq(main))
            .load::<PartTypes>(conn)?;

        Part::belonging_to(&types)
            .filter(owner.eq(user.get_id()))
            .filter(attached_to.is_null())
            .order_by(id)
            .load::<Part>(conn)
    }

    fn traverse (self, usage: &Usage, conn: &AppConn) -> QueryResult<Assembly> {
        let subs = Part::belonging_to(&self)
                .load::<Part>(conn)?
                .into_iter().map(|x| x.traverse(usage, conn))
                .collect::<QueryResult<Vec<_>>>()
                .map (|x| x.into_boxed_slice())?;
        let part = self.apply(usage, conn)?;

        Ok (Assembly {
            subs,
            part,
        })
    }

    pub fn register (usage: Usage, id: i32, user: &Person, conn: &AppConn) -> QueryResult<Assembly> {
        Part::get(id, user, conn)?
                .traverse (&usage, conn)
    }

    fn apply (mut self, usage: &Usage, conn: &AppConn) -> QueryResult<Part> {
        if let Some(func) = usage.op {
            info!("Applying usage to part {}", self.id);

            func(& mut self.time, usage.time);
            func(& mut self.distance, usage.distance);
            func(& mut self.climb, usage.climb);
            func(& mut self.descend, usage.descend);
            func(& mut self.count, 1);

            self.save_changes::<Part>(conn)
        } else {
            Ok(self)
        }
    }
 }

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(Part::types(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> DbResult<Json<Part>> {
    DbResult (Part::get(part, &user, &conn).map (|x| Json(x)))
}

#[get("/<part>?assembly")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> DbResult<Json<Assembly>> {
    DbResult (Part::register(Usage::none(), part, &user, &conn).map(|x| Json(x)))
}

#[get("/mygear")]
fn mygear(user: User, conn: AppDbConn) -> QueryResult<Json<Vec<Part>>> {    
    Part::part_by_user(&user, true, &conn).map(|x| Json(x))
}

#[get("/myspares")]
fn myspares(user: User, conn: AppDbConn) -> QueryResult<Json<Vec<Part>>> {    
    Part::part_by_user(&user, false, &conn).map(|x| Json(x))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, get_assembly, mygear, myspares]
}

#[cfg(test)]
mod test {
    use rocket::local::Client;
    use rocket::http::{Header, Status};
    use serde_json;
    use super::*;

    #[test]
    fn test_types () {
            let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

            let mut response = client.get("/part/types").header(Header::new("x-user-id", "2")).dispatch();
            assert_eq!(response.status(), Status::Ok);
            let types: Vec<PartTypes> = serde_json::from_str(&response.body_string().expect("")).expect("");
            assert_eq!(types.len(), 9);
            let t = &types[0];
            assert_eq!(t.id, 1);
            assert_eq!(t.name, "Bike");
            assert_eq!(types[0], PartTypes{id:1,name: String::from("Bike"), main:true, hooks: vec!(2,4,5,7,8)});
    }
    #[test]
    fn test_part () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/part/999").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let mut response = client.get("/part/myspares").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        
        let _myparts: Vec<Part> = serde_json::from_str(&response.body_string()
                .expect("body is no string")).expect("body is no part");

        response = client.get("/part/mygear").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let myparts: Vec<Part> = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no part");
        response = client.get(format!("/part/{}", myparts[0].id)).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let part: Part = serde_json::from_str(&response.body_string().expect("")).expect("");
        assert_eq!(part.name.to_string(), "Bronson");

        response = client.get(format!("/part/{}?assembly", myparts[1].id)).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let ass: Assembly = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no assembly");
        assert_eq!(ass.part.name.to_string(), "Slide");
    }
}