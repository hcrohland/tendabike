use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;

use crate::schema::{activities, activity_types};
use crate::user::*;
use crate::error::MyError;
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
   	pub time: Option<i32>,
    /// Covered distance
	pub distance: Option<i32>,
	/// Total climbing
    pub climb: Option<i32>,
    /// Total descending
	pub descend: Option<i32>,
    /// average power output
    pub power: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<i32>,
    registered: bool,
}

#[derive(Debug, Clone, Insertable, AsChangeset, PartialEq, Serialize, Deserialize)]
#[changeset_options(treat_none_as_null="true")]
#[table_name = "activities"]
pub struct NewActivity {
    pub user_id: i32,
    /// The activity type
    pub what: i32,
    /// This name of the activity.
    pub name: String,
    /// Start time
    pub start: DateTime<Utc>,
    /// End time
    pub duration: i32,
    /// activity time
   	pub time: Option<i32>,
    /// Covered distance
	pub distance: Option<i32>,
	/// Total climbing
    pub climb: Option<i32>,
    /// Total descending
	pub descend: Option<i32>,
    /// average power output
    pub power: Option<i32>,
    /// Which gear did she use?
    pub gear: Option<i32>,
}


impl Activity {
    fn types (conn: &AppConn) -> Vec<ActivityType> {
        activity_types::table.load::<ActivityType>(conn).expect("error loading ActivityTypes")
    }

    fn get(id: i32, person: &Person, conn: &AppConn) -> TbResult<Activity> {
        let act = activities::table.find(id).first::<Activity>(conn)?;
        if act.user_id != person.get_id() && !person.is_admin() {
                return Err(MyError::Forbidden(format!("User {} cannot access activity {}", person.get_id(), id)));
        }
        Ok(act)
    }

    fn delete(act_id: i32, person: &Person, conn: &AppConn) -> TbResult<Activity> {
        use crate::schema::activities::dsl::*;
        conn.transaction(|| {
            let mut act = Activity::get(act_id, person, conn)?;
            if act.gear != None {
                act.register(None, person, conn)?;
            }
            Ok(diesel::delete(activities.filter(id.eq(act_id))).get_result(conn)?)
        })
    }

    fn create(act: NewActivity, user: &Person, conn: &AppConn) -> TbResult<Activity> {
        if act.user_id != user.get_id() && !user.is_admin() {
            return Err(MyError::Forbidden(format!("user {} cannot create for user {}", user.get_id(), act.user_id)));
        }
        conn.transaction(|| {
            let mut new: Activity = diesel::insert_into(activities::table)
                .values(&act)
                .get_result(conn)?;
            if new.gear.is_some()  {
                new.register(None, user, conn)?; 
            }
            Ok(new)
        })
    }

    fn update (act_id: i32, act: NewActivity, user: &Person, conn: &AppConn) -> TbResult<Activity> {
        if act.user_id != user.get_id() && !user.is_admin() {
           return Err(MyError::Forbidden(format!("user {} cannot update for user {}", user.get_id(), act.user_id)));
        }
        conn.transaction(|| {
            let mut new: Activity = diesel::update(activities::table)
                .filter(activities::id.eq(act_id))
                .set(&act)
                .get_result(conn)?;
            if new.gear.is_some()  {
                new.register(None, user, conn)?; 
            }
            Ok(new)
        })
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

    fn register (& mut self, gear: Option<i32>, user: &Person, conn: &AppConn) -> TbResult<part::Assembly> {
        if self.gear == None && gear == None {
            return Err(MyError::AnyErr("trying to register to nothing...".to_string()));
        } 

        // unwrap_or_else evaluates lazily, contrary to unwrap_or!
        let new_gear = gear.unwrap_or_else(|| self.gear.unwrap());

        conn.transaction(|| {
            if self.registered == true {
                let gear = self.gear.unwrap();
                info!("de-registering activity {} from gear {}", self.id, gear);
                let part = part::Part::utilize(self.usage(std::ops::SubAssign::sub_assign), gear, user, conn)?;
                self.registered = false;
                if new_gear == 0 {
                    self.save_changes::<Activity>(conn)?;
                    return Ok(part);
                }
            } 
            
            info!("registering activity {} to gear {}", self.id, new_gear);
            self.registered = true;
            self.gear = Some(new_gear);
            self.save_changes::<Activity>(conn)?;
            part::Part::utilize(self.usage(std::ops::AddAssign::add_assign), new_gear, user, conn)
        })
    }

    fn scan (gear_id: i32, user: &Person, conn: &AppConn) -> TbResult<part::Assembly> {
        use crate::schema::activities::dsl::*;

        conn.transaction(|| {
            let mut acts = activities
                .filter(registered.eq(false)).filter(gear.eq(gear_id))
                .load::<Activity>(conn)?;

            acts.iter_mut()
                .map (|a| a.register(Some(gear_id), user, conn))
                .last().unwrap_or(Err(MyError::AnyErr("".to_string())))
        })
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(Activity::types(&conn))
}

#[get("/<id>")]
fn get (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Activity>> {
    Activity::get(id, &user, &conn).map(|x| Json(x))
}

use rocket::response::status;

#[post("/", data="<activity>")]
fn post (activity: Json<NewActivity>, user: User, conn: AppDbConn) 
            -> TbResult<status::Created<Json<Activity>>> {

    let activity = Activity::create(activity.0, &user, &conn)?;
    let url = uri! (get: activity.id);
    Ok (status::Created(url.to_string(), Some(Json(activity))))
}

#[put("/<id>", data="<activity>")]
fn put (id: i32, activity: Json<NewActivity>, user: User, conn: AppDbConn) -> TbResult<Json<Activity>> {
    Activity::update(id, activity.0, &user, &conn).map(|x| Json(x))
}

#[delete("/<id>")]
fn delete (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Activity>> {
    Activity::delete(id, &user, &conn).map(|x| Json(x))
}

#[patch("/<id>?<gear>")]
fn register (id: i32, gear: Option<i32>, user: User, conn: AppDbConn) -> TbResult<Json<part::Assembly>> {
    info! ("register act {} to gear {:?}", id, gear);
    
    Activity::get(id, &user, &conn)
        .map_or_else(
            |err| Err(err),
            |mut act| act.register(gear, &user, &conn).map(|x| Json(x))
        )
}

#[patch("/scan/<gear>")]
fn scan (gear: i32, user: User, conn: AppDbConn) -> TbResult<Json<part::Assembly>> {
    Activity::scan(gear, &user, &conn).map(|x| Json(x))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, register, scan, put, delete,post,]
}