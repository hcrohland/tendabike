use rocket_contrib::json::Json;

use crate::schema::{activity_types, part_types};
use crate::user::*;
use crate::*;

// use self::diesel::prelude::*;

use diesel::{self, QueryDsl, RunQueryDsl};

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartTypeId(i32);

NewtypeDisplay! { () pub struct PartTypeId(); }
NewtypeFrom! { () pub struct PartTypeId(i32); }

/// List of of all valid part types.
///
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
pub struct PartType {
    /// The primary key
    pub id: PartTypeId,
    /// The display name
    pub name: String,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: PartTypeId,
    /// Part types that can be attached
    pub hooks: Vec<PartTypeId>,
    /// the order for displaying types
    pub order: i32,
    /// Potential group
    pub group: Option<String>
}

#[derive(DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActTypeId(i32);

NewtypeDisplay! { () pub struct ActTypeId(); }
NewtypeFrom! { () pub struct ActTypeId(i32); }

/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, Identifiable, Queryable, PartialEq, Serialize, Deserialize)]
pub struct ActivityType {
    /// The primary key
    pub id: ActTypeId,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear_type: PartTypeId,
}

impl ActTypeId {
    pub fn get(self, conn: &AppConn) -> TbResult<ActivityType> {
        Ok(activity_types::table
            .find(self)
            .first::<ActivityType>(conn)?)
    }
}

#[get("/activity")]
fn activity(_user: &User, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(activity_types::table
        .order(activity_types::id)
        .load::<ActivityType>(&conn.0)
        .expect("error loading ActivityTypes"))
}

impl PartTypeId {
    pub fn get (self, conn: &AppConn) -> TbResult<PartType> {
        use schema::part_types::dsl::*;
        Ok(part_types
            .find(self)
            .get_result::<PartType>(conn)?)
    }

    fn filter_types(self, types: &mut Vec<PartType>) -> Vec<PartType> {
        let mut res = types
            .drain_filter(|x| x.hooks.contains(&self) || x.id == self)
            .collect::<Vec<_>>();
        for t in res.clone().iter() {
            res.append(&mut t.id.filter_types(types));
        }
        res
    }
    pub fn subtypes(self, conn: &AppConn) -> Vec<PartType> {
        use schema::part_types::dsl::*;
        let mut types = part_types
            .load::<PartType>(conn)
            .expect("Error loading parttypes");
        self.filter_types(&mut types)
    }
}

#[get("/part")]
fn part(conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(part_types::table
        .order(part_types::id)
        .load::<PartType>(&conn.0)
        .expect("error loading PartType"))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![part, activity]
}
