use std::collections::HashMap;

use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use self::schema::{parts, part_types, attachments};
use crate::user::*;
use crate::*;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

#[derive(DieselNewType)] 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)] 
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

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
    pub hooks: Vec<PartId>,
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
pub struct Part {
    /// The primary key
    pub id: PartId,
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
    /// usage count
    pub count: i32,
}

/// Timeline of attachments
/// 
/// Every attachement of a part to another part (hook) is an entry
/// Start and end time are noted
/// 
#[derive(Clone, Debug, PartialEq, 
        Serialize, Deserialize, 
        Queryable, Identifiable, Associations, AsChangeset)]
#[primary_key(id)]
#[belongs_to(Part, foreign_key = "hook_id")]
struct Attachment {
    // primary key
    pub id: i32,
    // the sub-part, which is attached to the hook
    pub part_id: PartId,
    // the hook, to which part_id is attached
    pub hook_id: PartId,
    // when it was attached
    pub attached: DateTime<Utc>,
    // when it was removed again
    pub detached: DateTime<Utc>,
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
pub type Assembly = HashMap<PartId, Part>;

pub trait ATrait {
    fn part (&self, part: PartId) -> Option<&Part>;
}

impl ATrait for Assembly {
    fn part (&self, part: PartId) -> Option<&Part> {
        self.get(&part)
    }
}

impl PartId {

    /// get the part with id part
    /// 
    /// Assumes authorization checked
    fn get (self, conn: &AppConn) -> TbResult<Part> {
        Ok(parts::table.find(self).first(conn)?)
    }

    /// check if the given user is the owner or an admin.
    /// Returns Forbidden if not.
    fn checkuser (self, user: &dyn Person, conn: &AppConn) -> TbResult<PartId> {
        use schema::parts::dsl::*;
        
        if user.is_admin() {
            return Ok(self)
        }

        let own = parts.find(self).filter(owner.eq(user.get_id())).select(owner).first::<i32>(conn)?;
        if user.get_id() == own {
            return Ok(self);
        }

        Err(MyError::Forbidden(format!("user {} cannot access part {}", user.get_id(), self)))
    }

    /// apply a usage to the part with given id
    /// 
    /// returns the changed part
    fn apply (self, usage: &Usage, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        info!("Applying usage to part {}", self);
        Ok(diesel::update(parts.find(self))
            .set((  time.eq(time + usage.time),
                    climb.eq(climb + usage.climb),
                    descend.eq(descend + usage.descend),
                    distance.eq(distance + usage.distance),
                    count.eq(count + usage.count)))
            .get_result::<Part>(conn)?)
    }

    /// Retrieve the part_id self is attached to or none
    /// 
    /// panics on unexpected database behaviour
    fn attached_to(self, at_time: DateTime<Utc>, conn: &AppConn) -> Option<PartId> {
        use schema::attachments::dsl::*;

        match attachments.select(hook_id) 
            .filter(part_id.eq(self)) 
            .filter(attached.lt(at_time)).filter(detached.ge(at_time))
            .first::<PartId>(conn) {
                Ok(part) => Some(part),
                Err(diesel::result::Error::NotFound) => None,
                _ => panic!("Could not read attachments")
            }
    }

    /// retrieve the vector of Subparts for self
    /// 
    /// panics on unexpected database error
    fn subparts(self, at_time: DateTime<Utc>, conn: &AppConn) -> Vec<PartId> {
        use schema::attachments::dsl::*;

        attachments.select(part_id)
            .filter(hook_id.eq(self))
            .filter(attached.lt(at_time)).filter(detached.ge(at_time))
            .load::<PartId>(conn).expect("could not read attachments")
    }

    fn traverse (self, map: & mut Assembly, usage: &Usage, conn: &AppConn) -> TbResult<()> {
        self.subparts(usage.start, conn)
                .into_iter().map(|x| x.traverse(map, usage, conn))
                .for_each(drop);

        map.insert(self, self.apply(usage, conn)?);
        Ok(())
    }


    pub fn utilize (self, map: & mut Assembly, usage: Usage, user: &dyn Person, conn: &AppConn) -> TbResult<()> {
        self.checkuser(user, conn)?.traverse(map, &usage, conn)
    }
}

impl Part {
    /// list all part types
    fn types (conn: &AppConn) -> Vec<PartTypes> {
        part_types::table.order_by(part_types::id).load::<PartTypes>(conn).expect("error loading PartTypes")
    }

    /// retrieve the list of available parts for a user
    /// 
    /// it only returns parts which are not attached
    /// if parameter main is true it returns all gear, which can be used for activities
    /// If parameter main is false it returns the list of spares which can be attached to gear
    fn parts_by_user (user: &dyn Person, main: bool, conn: &AppConn) -> TbResult<Vec<Part>>{
        use crate::schema::parts::dsl::*;

        let types = part_types::table
            .filter(part_types::main.eq(main))
            .load::<PartTypes>(conn)?;

        let plist = Part::belonging_to(&types) // only gear or spares
            .filter(owner.eq(user.get_id()))
            .order_by(id)
            .load::<Part>(conn)?;
        Ok(plist.into_iter()
            .filter(|x| {
                x.id.attached_to(Utc::now(), conn).is_none() // only parts which are not attached
            }).collect())
    }

    pub fn reset (user: &dyn Person, conn: &AppConn) -> TbResult<Vec<PartId>> {
        use schema::parts::dsl::*;
        use std::collections::HashSet;
        
        let part_list = diesel::update(parts.filter(owner.eq(user.get_id())))
            .set((  time.eq(0),
                    climb.eq(0),
                    descend.eq(0),
                    distance.eq(0),
                    count.eq(0)))
            .get_results::<Part>(conn)?;

        let mains: HashSet<i32> = part_types::table.select(part_types::id).filter(part_types::main.eq(true))
            .load::<i32>(conn).expect("error loading PartTypes").into_iter().collect();

        Ok(part_list.into_iter()
            .filter(|x| mains.contains(&x.what)).map(|x| x.id)
            .collect())
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(Part::types(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Part>> {
    PartId(part).checkuser(&user, &conn)?
        .get(&conn).map (Json)
}

#[get("/<part>?assembly")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    let mut map = Assembly::new();

    PartId(part).checkuser(&user, &conn)?
        .traverse(&mut map, &Usage::none(), &conn)?;
    
    Ok(Json(map))
}

#[get("/mygear")]
fn mygear(user: User, conn: AppDbConn) -> TbResult<Json<Vec<Part>>> {    
    Part::parts_by_user(&user, true, &conn).map(Json)
}

#[get("/myspares")]
fn myspares(user: User, conn: AppDbConn) -> TbResult<Json<Vec<Part>>> {    
    Part::parts_by_user(&user, false, &conn).map(Json)
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, get_assembly, mygear, myspares]
}