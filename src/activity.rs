use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use crate::schema::*;
use crate::user::*;
use crate::*;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};


/// The list of activity types
/// Includes the kind of gear which can be used for this activity
/// multiple gears are possible
#[derive(Debug, Clone, Identifiable, Queryable, PartialEq, Serialize)]
pub struct ActivityType {
    /// The primary key
    pub id: i32,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear: i32,
}


/// The database's representation of an activity.
#[derive(Debug, Clone, Identifiable, Queryable, PartialEq, Serialize, AsChangeset)]
#[table_name = "activities"]
pub struct Activity {
    /// The primary key
    pub id: i32,
    /// The athlete
    pub user_id: i32,
    /// The activity type
    pub what: Option<i32>,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub duration: i32,
    /// activity time
   	time: Option<i32>,
    /// Covered distance
	distance: Option<i32>,
	/// Total climbing
    climb: Option<i32>,
    /// Total descending
	descend: Option<i32>,
    /// average power output
    power: Option<i32>,
    /// Which gear did she use?
    gear: Option<i32>,
    registered: bool,
}

impl Activity {
    fn types (conn: &AppConn) -> Vec<ActivityType> {
        activity_types::table.load::<ActivityType>(conn).expect("error loading ActivityTypes")
    }

    fn get(id: i32, _user: &User, conn: &AppConn) -> Option<Activity> {
        activities::table.find(id).first::<Activity>(conn).ok()
    }

    fn usage (&self, factor: i32) -> Usage {
        Usage {
            start: self.start,
            time: self.time.unwrap_or(0) * factor,
            distance: self.distance.unwrap_or(0) * factor,
            climb: self.climb.unwrap_or(0) * factor,
            descend: self.descend.unwrap_or(self.climb.unwrap_or(0)) * factor,
            power: self.power.unwrap_or(0) * factor,            
        }
    }

    fn register (mut self, gear: Option<i32>, user: &User, conn: &AppConn) -> Option<()> {
        if self.registered == true {
            part::Part::register(self.usage(-1), self.gear?, user, conn)?;
            self.registered = false;
        }
        
        let gear = gear.unwrap_or(self.gear?);

        part::Part::register(self.usage(1), gear, user, conn)?;
        self.registered = true;
        self.gear = Some(gear);
        self.save_changes::<Activity>(conn).expect("saving changes to Activity failed");
        Some(())
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(Activity::types(&conn))
}

#[get("/<id>")]
fn get (id: i32, user: User, conn: AppDbConn) -> Option<Json<Activity>> {
    Activity::get(id, &user, &conn).map(|x| Json(x))
}

#[patch("/<id>?bike&<gear>")]
fn register (id: i32, gear: Option<i32>, user: User, conn: AppDbConn) -> Option<()> {
    Activity::get(id, &user, &conn)?.register(gear, &user, &conn)
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, register,]
}