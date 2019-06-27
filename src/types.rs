use rocket_contrib::json::Json;

use crate::schema::{activity_types, part_types};
use crate::*;
use crate::user::*;

// use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

#[derive(DieselNewType)] 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)] 
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
    /// Part types that can be attached
    pub hooks: Vec<PartId>,
    /// is it a main part? I.e. can it be used for an activity?
    pub main: bool,
}

#[derive(DieselNewType)] 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)] 
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
        Ok(activity_types::table.find(self).first::<ActivityType>(conn)?)
    }
}

#[get("/activity")]
fn activity(_user: User, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(activity_types::table.order(activity_types::id).load::<ActivityType>(&conn.0).expect("error loading ActivityTypes"))
}


#[get("/part")]
fn part(_user: User, conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(part_types::table.order(part_types::id).load::<PartType>(&conn.0).expect("error loading PartType"))
}


pub fn routes () -> Vec<rocket::Route> {
    routes![part, activity]
}