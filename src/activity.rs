
use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;
use rocket::response::status;

use crate::schema::activities;
use crate::user::*;
use crate::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

use part::Assembly;
use part::ATrait;


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
    fn read(self, person: &dyn Person, conn: &AppConn) -> TbResult<Activity> {
        let act = activities::table.find(self).for_update().first::<Activity>(conn)?;
        if act.user_id != person.get_id() && !person.is_admin() {
                return Err(MyError::Forbidden(format!("User {} cannot access activity {}", person.get_id(), self)));
        }
        Ok(act)
    }

    fn delete(self, person: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        use crate::schema::activities::dsl::*;
        conn.transaction(|| {
            let res = self.read(person, conn)?
                        .register(None, person, conn)?;
            diesel::delete(activities.filter(id.eq(self))).execute(conn)?;
            Ok (res)
        })
    }

    fn update (self, act: NewActivity, user: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction(|| {
            let mut hash = Assembly::new();

            let old = self.read(user, conn)?;
            if let Some(gear) = old.gear {
                gear.utilize(&mut hash, old.usage(), -1, user, conn)?;
            }

            let new: Activity = diesel::update(activities::table)
                .filter(activities::id.eq(self))
                .set(&act)
                .get_result(conn)?;
            if let Some(gear) = new.gear {
                gear.utilize(&mut hash, new.usage(), 1, user, conn)?;
            }
            
            new.check_geartype(hash, conn)
        })
    }

}

impl Activity {
    /// check if the gear is allowed for the activity
    fn check_geartype(&self, ass: Assembly, conn: &AppConn) -> TbResult<Assembly> {
        let gear_id = match self.gear {
            Some(x) => x,
            None => return Ok(ass)
        };
        let mygear = match ass.part(gear_id) {
            Some (x) => x,
            None => return Err(MyError::AnyErr("Main gear not found in assembly".to_string()))
        };

        if mygear.what == self.what.get(conn)?.gear_type {
            Ok(ass) } 
        else {
            Err(MyError::BadRequest(format!("Gear type {} cannot be used for activity type {}", mygear.what, self.what)))
        } 
    }

    /// create a new activity
    /// 
    /// - returns the activity and all affected parts
    /// - checks authorization
    fn create(act: NewActivity, user: &dyn Person, conn: &AppConn) -> TbResult<(Activity, Assembly)> {
        if act.user_id != user.get_id() && !user.is_admin() {
            return Err(MyError::Forbidden(format!("user {} cannot create activity for user {}", user.get_id(), act.user_id)));
        }
        conn.transaction(|| {
            let new: Activity = diesel::insert_into(activities::table)
                .values(&act)
                .get_result(conn)?;
            let mut res = Assembly::new();
            if let Some(gear) = new.gear {
                gear.utilize(&mut res, new.usage(), 1, user, conn)?;
            }
            let res = new.check_geartype(res, conn)?;
            Ok((new, res))
        })
    }

    /// Extract the usage out of an activity
    /// 
    /// If the descend value is missing, assume descend = climb
    pub fn usage (&self) -> Usage {
        Usage {
            start: self.start,
            time: self.time.unwrap_or(0),
            distance: self.distance.unwrap_or(0),
            climb: self.climb.unwrap_or(0),
            descend: self.descend.unwrap_or_else(|| self.climb.unwrap_or(0)),
            power: self.power.unwrap_or(0),  
            count: 1,          
        }
    }

    /// register a (new) gear to an activity 
    /// 
    /// if there is already an gear attached, automatically deregister the old gear
    /// returns all affected parts
    fn register (& mut self, gear: Option<PartId>, user: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction(|| {
            let mut hash = Assembly::new();

            if self.gear == gear {
                return Ok(hash)
            }

            if let Some(gear) = self.gear {
                // info!("de-registering activity {} from gear {}", self.id, gear);
                self.gear = None;
                gear.utilize(&mut hash, self.usage(), -1, user, conn)?;
            } 
            if let Some(new_gear) = gear {
                // info!("registering activity {} to gear {}", self.id, new_gear);
                self.gear = gear;
                new_gear.utilize(&mut hash, self.usage(), 1, user, conn)?;
            }
            
            self.save_changes::<Activity>(conn)?;
            // we do optimistic checking
            // now we have all data to check if we are allowed to do this
            // if the check fails the database will roll back!
            self.check_geartype(hash, conn)
        })
    }

    /// find all activities for gear part in the given time frame
    /// 
    /// if end is none it means for the whoel future
    pub fn find (part: PartId, begin: DateTime<Utc>, end: Option<DateTime<Utc>>, conn: &AppConn) -> Vec<Activity> {
        use schema::activities::dsl::{activities,gear,start};

        let mut query = activities.filter(gear.eq(Some(part)))
                        .filter(start.ge(begin)).into_boxed();
        if let Some(end) = end { query = query.filter(start.lt(end)) }
        query.load::<Activity>(conn).expect("could not read activities")
    }

    /// rescan all activites for a user
    /// 
    /// This will correct the utilization data for that user
    ///  as long as no other users used her gear...
    fn rescan (user: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction(|| {
            let main_gears = part::Part::reset(user, conn)?;

            let mut ass = Assembly::new();
            let activities = activities::table.filter(activities::gear.eq_any(main_gears))
                .load::<Activity>(conn)?;

            for act in activities {
                act.gear.unwrap().utilize(&mut ass, act.usage(), 1, user, conn)?
            }
            Ok(ass)
        })
    }
}

#[get("/<id>")]
fn get (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Activity>> {
    ActivityId(id).read(&user, &conn).map(Json)
}

#[post("/", data="<activity>")]
fn post (activity: Json<NewActivity>, user: User, conn: AppDbConn) 
            -> TbResult<status::Created<Json<(Activity, Assembly)>>> {

    let (activity, assembly) = Activity::create(activity.0, &user, &conn)?;
    let id_raw: i32 = activity.id.into();
    let url = uri! (get: id_raw);
    Ok (status::Created(url.to_string(), Some(Json((activity, assembly)))))
}

#[put("/<id>", data="<activity>")]
fn put (id: i32, activity: Json<NewActivity>, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    ActivityId(id).update(activity.0, &user, &conn).map(Json)
}

#[delete("/<id>")]
fn delete (id: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    ActivityId(id).delete(&user, &conn).map(Json)
}

#[patch("/<id>?<gear>")]
fn register (id: i32, gear: Option<i32>, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    info! ("register act {} to gear {:?}", id, gear);
    let gear = match gear {
            None => None,
            Some(x) => Some(PartId::get(x, &user, &conn)?)
        };
    conn.transaction(|| {
        ActivityId(id).read(&user, &conn)
            .map_or_else(
                Err,
                |mut act| act.register(gear, &user, &conn).map(Json)
            )
    })
}

#[patch("/rescan")]
fn rescan (user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    Ok(Json(Activity::rescan(&user, &conn)?))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![get, register, put, delete, post, rescan]
}