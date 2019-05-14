use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use crate::schema::{activities, activity_types};
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
#[derive(Debug, Clone, Identifiable, Queryable, PartialEq, Serialize, Deserialize)]
pub struct ActivityType {
    /// The primary key
    pub id: i32,
    /// The name
    pub name: String,
    /// Gears which can be used for this activity type
    pub gear: i32,
}


/// The database's representation of an activity.
#[derive(Debug, Clone, Identifiable, Queryable, AsChangeset, PartialEq, Serialize, Deserialize)]
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

    fn get(id: i32, _user: &Person, conn: &AppConn) -> QueryResult<Activity> {
        activities::table.find(id).first::<Activity>(conn)
    }

    fn usage (&self, op: for<'r> fn(&'r mut i32, i32)) -> Usage {
        Usage {
            op: Some(op),
            start: self.start,
            time: self.time.unwrap_or(0),
            distance: self.distance.unwrap_or(0),
            climb: self.climb.unwrap_or(0),
            descend: self.descend.unwrap_or(self.climb.unwrap_or(0)),
            power: self.power.unwrap_or(0),            
        }
    }

    fn register (mut self, gear: Option<i32>, user: &Person, conn: &AppConn) -> QueryResult<part::Assembly> {
        if self.gear == None && gear == None {
            return Err(diesel::NotFound);
        } 

        // unwarp_or_else evaluates lazily, contrary to unwrap_or!
        let gear = gear.unwrap_or_else(|| self.gear.unwrap());

        conn.transaction(|| {
            if self.registered == true {
                let gear = self.gear.unwrap();
                info!("de-registering activity {} from gear {}", self.id, gear);
                part::Part::register(self.usage(std::ops::SubAssign::sub_assign), gear, user, conn)?;
                self.registered = false;
            } 
            
            info!("registering activity {} to gear {}", self.id, gear);
            self.registered = true;
            self.gear = Some(gear);
            self.save_changes::<Activity>(conn)?;
            part::Part::register(self.usage(std::ops::AddAssign::add_assign), gear, user, conn)
        })
    }

    fn update (gear_id: i32, user: &Person, conn: &AppConn) -> QueryResult<part::Assembly> {
        use crate::schema::activities::dsl::*;

        conn.transaction(|| {
            let acts = activities
                .filter(registered.eq(false)).filter(gear.eq(gear_id))
                .load::<Activity>(conn)?;

            let mut res = Err(diesel::NotFound);
            for a in acts  {
                res = a.register (Some(gear_id), user, conn);
            };
            res
        })
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(Activity::types(&conn))
}

#[get("/<id>")]
fn get (id: i32, user: User, conn: AppDbConn) -> DbResult<Json<Activity>> {
    DbResult(Activity::get(id, &user, &conn).map(|x| Json(x)))
}

#[patch("/<id>?<gear>")]
fn register (id: i32, gear: Option<i32>, user: User, conn: AppDbConn) -> DbResult<Json<part::Assembly>> {
    info! ("register act {} to gear {:?}", id, gear);
    
    Activity::get(id, &user, &conn)
        .map_or_else(
            |err| DbResult(Err(err)),
            |act| DbResult (act.register(gear, &user, &conn).map(|x| Json(x)))
        )
}

#[patch("/update/<gear>")]
fn update (gear: i32, user: User, conn: AppDbConn) -> DbResult<Json<part::Assembly>> {
    DbResult (Activity::update(gear, &user, &conn).map(|x| Json(x)))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, register, update, ]
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

            let mut response = client.get("/activ/types").header(Header::new("x-user-id", "2")).dispatch();
            assert_eq!(response.status(), Status::Ok);
            let types: Vec<ActivityType> = serde_json::from_str(&response.body_string().expect("")).expect("");
            assert!(types.len() > 0);
            assert_eq!(types[0], ActivityType {id:1,name: String::from("Bike Ride"), gear: 1});
    }
    #[test]
    fn test_activities () {
        let client = Client::new(crate::ignite_rocket()).expect("valid rocket instance");

        let response = client.get("/activ/999").header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::NotFound);

        let mut response = client.get(format!("/activ/1")).header(Header::new("x-user-id", "2")).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let _part: Activity = serde_json::from_str(&response.body_string()
            .expect("body is no string")).expect("body is no activity");

    }
}