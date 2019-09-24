//! Activity handling for the TendaBike backend
//! 
//! struct Activity captures all data of an athlete's activity
//! 
//! By assigning a gear to the activity it gets accounted with that gear and all it's parts attached 
//! at the start time of the activity  
//! Most operations are done on the ActivityId though  
//! 
use chrono::{
    Utc,
    DateTime,
};

use std::collections::HashMap;

use rocket_contrib::json::Json;
use rocket::response::status;

use crate::attachment;
use crate::schema::activities;
use crate::user::*;
use crate::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

/// The Id of an Activity
/// 
/// Most operations for activities are done on the Id alone
/// 
#[derive(DieselNewType)] 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)] 
pub struct ActivityId(i32);

NewtypeDisplay! { () pub struct ActivityId(); }
NewtypeFrom! { () pub struct ActivityId(i32); }

/// The database's representation of an activity.
#[derive(Debug, Clone, Identifiable, Queryable, AsChangeset, PartialEq, Serialize, Deserialize)]
#[changeset_options(treat_none_as_null="true")]
#[table_name = "activities"]
pub struct Activity {
    /// The primary key
    pub id: ActivityId,
    /// The athlete
    pub user_id: i32,
    /// The activity type
    pub what: ActTypeId,
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
    pub gear: Option<PartId>,
}

#[derive(Debug, Clone, Insertable, AsChangeset, PartialEq, Serialize, Deserialize)]
#[changeset_options(treat_none_as_null="true")]
#[table_name = "activities"]
pub struct NewActivity {
    pub user_id: i32,
    /// The activity type
    pub what: ActTypeId,
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
    pub gear: Option<PartId>,
}

impl ActivityId {

    /// Read the activity with id self
    /// 
    /// checks authorization
    fn read(self, person: &dyn Person, conn: &AppConn) -> TbResult<Activity> {
        let act = activities::table.find(self).for_update().first::<Activity>(conn)?;
        if act.user_id != person.get_id() && !person.is_admin() {
                return Err(MyError::Forbidden(format!("User {} cannot access activity {}", person.get_id(), self)));
        }
        Ok(act)
    }

    /// Delete the activity with id self
    /// and update part usage accordingly 
    /// 
    /// returns all affected parts  
    /// checks authorization  
    fn delete(self, person: &dyn Person, conn: &AppConn) -> TbResult<PartList> {
        use crate::schema::activities::dsl::*;
        conn.transaction(|| {
            let res = self.read(person, conn)?
                         .register(Factor::Sub, conn)?;
            diesel::delete(activities.filter(id.eq(self))).execute(conn)?;
            Ok (res)
        })
    }

    /// Update the activity with id self according to the data in NewActivity
    /// and update part usage accordingly 
    ///
    /// returns all affected parts  
    /// checks authorization  
    fn update (self, act: NewActivity, user: &dyn Person, conn: &AppConn) -> TbResult<PartList> {
        conn.transaction(|| {
            let mut res: HashMap<_, _> = 
                self.read(user, conn)?.register(Factor::Sub, conn)?
                    .into_iter().map(|x| (x.id, x))
                    .collect();

            let res2 = diesel::update(activities::table)
                .filter(activities::id.eq(self))
                .set(&act)
                .get_result::<Activity>(conn)?
                .register(Factor::Add, conn)?;

            for part in res2 {
                res.insert(part.id, part);
            }
            Ok(res.into_iter().map(|(_, part)| part).collect())
        })
    }

}

impl Activity {
    /// create a new activity
    /// 
    /// returns the activity and all affected parts  
    /// checks authorization  
    fn create(act: NewActivity, user: &dyn Person, conn: &AppConn) -> TbResult<(Activity, PartList)> {
        if act.user_id != user.get_id() && !user.is_admin() {
            return Err(MyError::Forbidden(format!("user {} cannot create activity for user {}", user.get_id(), act.user_id)));
        }
        conn.transaction(|| {
            let new: Activity = diesel::insert_into(activities::table)
                .values(&act)
                .get_result(conn)?;
            // let res = new.check_geartype(res, conn)?;
            let parts = new.register(Factor::Add, conn)?;
            Ok((new, parts))
        })
    }

    /// Extract the usage out of an activity
    /// 
    /// If the descend value is missing, assume descend = climb
    /// Account for Factor
    pub fn usage (&self, factor: Factor) -> Usage {
        let factor = factor as i32;
        Usage {
            time: self.time.unwrap_or(0) * factor,
            distance: self.distance.unwrap_or(0) * factor,
            climb: self.climb.unwrap_or(0) * factor,
            descend: self.descend.unwrap_or_else(|| self.climb.unwrap_or(0)) * factor,
            power: self.power.unwrap_or(0) * factor,  
            count: factor,          
        }
    }

    /// find all activities for gear part in the given time frame
    /// 
    /// if end is none it means for the whole future
    pub fn find (part: PartId, begin: DateTime<Utc>, end: Option<DateTime<Utc>>, conn: &AppConn) -> Vec<Activity> {
        use schema::activities::dsl::{activities,gear,start};

        let mut query = activities.filter(gear.eq(Some(part)))
                        .filter(start.ge(begin)).into_boxed();
        if let Some(end) = end { query = query.filter(start.lt(end)) }
        query.load::<Activity>(conn).expect("could not read activities")
    }

    fn register (&self, factor: Factor, conn: &AppConn) -> TbResult<PartList> {
        attachment::parts_per_activity(self, conn).iter()
            .map(|x| x.apply(&self.usage(factor), conn)).collect()
    }
}


/// web interface to read an activity
#[get("/<id>")]
fn get (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Activity>> {
    ActivityId(id).read(&user, &conn).map(Json)
}

/// web interface to create an activity
#[post("/", data="<activity>")]
fn post (activity: Json<NewActivity>, user: User, conn: AppDbConn) 
            -> TbResult<status::Created<Json<(Activity, PartList)>>> {

    let (activity, assembly) = Activity::create(activity.0, &user, &conn)?;
    let id_raw: i32 = activity.id.into();
    let url = uri! (get: id_raw);
    Ok (status::Created(url.to_string(), Some(Json((activity, assembly)))))
}

/// web interface to change an activity
#[put("/<id>", data="<activity>")]
fn put (id: i32, activity: Json<NewActivity>, user: User, conn: AppDbConn) -> TbResult<Json<PartList>> {
    ActivityId(id).update(activity.0, &user, &conn).map(Json)
}

/// web interface to delete an activity
#[delete("/<id>")]
fn delete (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<PartList>> {
    ActivityId(id).delete(&user, &conn).map(Json)
}

pub fn routes () -> Vec<rocket::Route> {
    routes![get, put, delete, post]
}