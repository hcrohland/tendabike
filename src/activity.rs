//use std::collections::HashMap;

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

use part::Assembly;


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
#[changeset_options(treat_none_as_null="true")]
#[table_name = "activities"]
pub struct Activity {
    /// The primary key
    pub id: i32,
    /// The athlete
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

    fn delete(act_id: i32, person: &Person, conn: &AppConn) -> TbResult<Assembly> {
        use crate::schema::activities::dsl::*;
        conn.transaction(|| {
            let mut act = Activity::get(act_id, person, conn)?;
            let res = act.register(None, person, conn)?;
            diesel::delete(activities.filter(id.eq(act_id))).execute(conn)?;
            Ok (res)
        })
    }

    fn check_geartype(&self, ass: Assembly, conn: &AppConn) -> TbResult<Assembly> {
        use crate::schema::activity_types::dsl::*;
        let actt = activity_types.find(self.what).first::<ActivityType>(conn)?;
        let gear_id = match self.gear {
            Some(x) => x,
            None => return Ok(ass)
        };
        let mygear = match ass.get(&gear_id) {
            Some (x) => x,
            None => return Err(MyError::AnyErr("Main gear not found in assembly".to_string()))
        };

        if mygear.what == actt.gear {
            Ok(ass)
        } else {
            Err(MyError::BadRequest(
                    format!("Gear type {} cannot be used for activity type {}", 
                                mygear.what, self.what)))
        }   
    }

    fn create(act: NewActivity, user: &Person, conn: &AppConn) -> TbResult<(Activity, Assembly)> {
        if act.user_id != user.get_id() && !user.is_admin() {
            return Err(MyError::Forbidden(format!("user {} cannot create for user {}", user.get_id(), act.user_id)));
        }
        conn.transaction(|| {
            let new: Activity = diesel::insert_into(activities::table)
                .values(&act)
                .get_result(conn)?;
            let mut res = Assembly::new();
            if let Some(gear) = new.gear {
                part::Part::utilize(&mut res, new.usage(std::ops::AddAssign::add_assign), gear, user, conn)?;
            }
            let res = new.check_geartype(res, conn)?;
            Ok((new, res))
        })
    }

    fn update (act_id: i32, act: NewActivity, user: &Person, conn: &AppConn) -> TbResult<Assembly> {
        use std::ops::{SubAssign,AddAssign};

        conn.transaction(|| {
            let mut hash = Assembly::new();

            let old = Activity::get(act_id, user, conn)?;
            if let Some(gear) = old.gear {
                part::Part::utilize(&mut hash, old.usage(SubAssign::sub_assign), gear, user, conn)?;
            }

            let new: Activity = diesel::update(activities::table)
                .filter(activities::id.eq(act_id))
                .set(&act)
                .get_result(conn)?;
            if let Some(gear) = new.gear {
                part::Part::utilize(&mut hash, new.usage(AddAssign::add_assign), gear, user, conn)?;
            }
            
            new.check_geartype(hash, conn)
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
            count: 1,          
        }
    }

    fn register (& mut self, gear: Option<i32>, user: &Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction(|| {
            let mut hash = Assembly::new();

            if self.gear == gear {
                return Ok(hash)
            }

            if let Some(gear) = self.gear {
                info!("de-registering activity {} from gear {}", self.id, gear);
                self.gear = None;
                part::Part::utilize(&mut hash, self.usage(std::ops::SubAssign::sub_assign), gear, user, conn)?;
            } 
            if let Some(new_gear) = gear {
                info!("registering activity {} to gear {}", self.id, new_gear);
                self.gear = gear;
                part::Part::utilize(&mut hash, self.usage(std::ops::AddAssign::add_assign), new_gear, user, conn)?;
            }
            
            self.save_changes::<Activity>(conn)?;
            self.check_geartype(hash, conn)
        })
    }

    fn rescan (part: i32, ass: &mut Assembly, user: &Person, conn: &AppConn) -> TbResult<()> {
        let activities = activities::table.filter(activities::gear.eq(part)).load::<Activity>(conn)?;

        for act in activities {
            part::Part::utilize(ass, act.usage(std::ops::AddAssign::add_assign), part, user, conn)?
        }
        Ok(())
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
            -> TbResult<status::Created<Json<(Activity, Assembly)>>> {

    let (activity, assembly) = Activity::create(activity.0, &user, &conn)?;
    let url = uri! (get: activity.id);
    Ok (status::Created(url.to_string(), Some(Json((activity, assembly)))))
}

#[put("/<id>", data="<activity>")]
fn put (id: i32, activity: Json<NewActivity>, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    Activity::update(id, activity.0, &user, &conn).map(|x| Json(x))
}

#[delete("/<id>")]
fn delete (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    Activity::delete(id, &user, &conn).map(|x| Json(x))
}

#[patch("/<id>?<gear>")]
fn register (id: i32, gear: Option<i32>, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    info! ("register act {} to gear {:?}", id, gear);
    
    Activity::get(id, &user, &conn)
        .map_or_else(
            |err| Err(err),
            |mut act| act.register(gear, &user, &conn).map(|x| Json(x))
        )
}

#[patch("/all/<part>")]
fn rescan (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    let mut map = Assembly::new();

    part::Part::utilize(&mut map, Usage::reset(), part, &user, &conn)?;
    Activity::rescan(part, &mut map, &user, &conn)?;
    
    Ok(Json(map))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![types, get, register, put, delete, post, rescan]
}