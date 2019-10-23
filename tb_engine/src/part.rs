use std::collections::HashMap;

use chrono::{
    Utc,
    DateTime,
};

use rocket_contrib::json::Json;
use rocket::response::status;

use self::schema::{parts,part_types};
use crate::user::*;
use crate::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

// make rls happy for now. This is broken anyways...

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

/// The database's representation of a part. 
#[derive(Clone, Debug, PartialEq, 
        Serialize, Deserialize, 
        Queryable, Identifiable, Associations, AsChangeset)]
#[primary_key(id)]
#[table_name = "parts"]
#[belongs_to(PartType, foreign_key = "what")]
pub struct Part {
    /// The primary key
    pub id: PartId,
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: PartTypeId,
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

#[derive(Clone, Debug, PartialEq, 
        Serialize, Deserialize, 
        Insertable)]
#[table_name = "parts"]
pub struct NewPart {
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: PartTypeId,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
}

#[derive(DieselNewType)] 
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)] 
pub struct PartId(i32);

NewtypeDisplay! { () pub struct PartId(); }
NewtypeFrom! { () pub struct PartId(i32); }

impl PartId {
    pub fn get (id: i32, user: &dyn Person, conn: &AppConn) -> TbResult<PartId> {
        PartId(id).checkuser(user, conn)
    }

    /// get the part with id part
    pub fn part (self, user: &dyn Person, conn: &AppConn) -> TbResult<Part> {
        let part = parts::table.find(self).first::<Part>(conn).with_context(|| format!("part {} does not exist", self))?;
        user.check_owner(part.owner, format!("user {} cannot access part {}", user.get_id(), part.id))?;
        Ok(part)
    }

    pub fn what (self, user: &dyn Person, conn: &AppConn) -> TbResult<PartTypeId> {
        Ok(self.part(user, conn)?.what)
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

        bail!(Error::NotFound(format!("user {} cannot access part {}", user.get_id(), self)))
    }

    /// apply a usage to the part with given id
    /// 
    /// returns the changed part
    pub fn apply (self, usage: &Usage, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        info!("Applying usage to part {}", self);
        // If the purchase date is newer than the usage, adjust it
        diesel::update(parts.find(self))
            .filter(purchase.gt(usage.start))
            .set(purchase.eq(usage.start))
            .execute(conn)?;
        Ok(diesel::update(parts.find(self))
            .set((  time.eq(time + usage.time),
                    climb.eq(climb + usage.climb),
                    descend.eq(descend + usage.descend),
                    distance.eq(distance + usage.distance),
                    count.eq(count + usage.count)))
            .get_result::<Part>(conn)?)
    }
}

impl Part {

    /// retrieve the list of available parts for a user
    /// 
    /// it only returns parts which are not attached
    /// if parameter main is true it returns all gear, which can be used for activities
    /// If parameter main is false it returns the list of spares which can be attached to gear
    fn parts_by_user (user: &dyn Person, main: bool, conn: &AppConn) -> TbResult<PartList>{
        use crate::schema::parts::dsl::*;

        let types = if main {
            part_types::table
                .filter(part_types::main.eq(part_types::id))
                .load::<PartType>(conn)?
        } else {
            part_types::table
                .filter(part_types::main.ne(part_types::id))
                .load::<PartType>(conn)?
        };

        let plist = Part::belonging_to(&types) // only gear or spares
            .filter(owner.eq(user.get_id()))
            .order_by(id)
            .load::<Part>(conn)?;
        Ok(plist.into_iter()
            .filter(|x| {
                attachment::is_attached(x.id, Utc::now(), conn).is_none() // only parts which are not attached
            }).collect())
    }

    /// retrieve the vector of Subparts for self
    /// 
    /// panics on unexpected database error
    fn subparts(& self, at_time: DateTime<Utc>, conn: &AppConn) -> PartList {
        attachment::subparts(self, at_time, conn)
    }

    /// reset all usage counters for all parts of a person
    /// 
    /// returns the list of main gears affected
    pub fn reset (user: &dyn Person, conn: &AppConn) -> TbResult<Vec<PartId>> {
        use schema::parts::dsl::*;
        use std::collections::HashSet;
        
        // reset all counters for all parts of this user
        let part_list = diesel::update(parts.filter(owner.eq(user.get_id())))
            .set((  time.eq(0),
                    climb.eq(0),
                    descend.eq(0),
                    distance.eq(0),
                    count.eq(0)))
            .get_results::<Part>(conn)?;

        // get the main types
        let mains: HashSet<PartTypeId> = part_types::table.select(part_types::id).filter(part_types::main.eq(part_types::id))
            .load::<PartTypeId>(conn).expect("error loading PartType").into_iter().collect();

        // only return the main parts
        Ok(part_list.into_iter()
            .filter(|x| mains.contains(&x.what)).map(|x| x.id)
            .collect())
    }
}

fn assembly (parts: &mut Vec<(PartTypeId, Part)>, at_time: DateTime<Utc>, user: &dyn Person, conn: &AppConn) { 
    for (_, part) in parts.clone() {
        let mut subs = std::iter::repeat(part.what)
                            .zip(part.subparts(at_time, conn).into_iter()).collect::<Vec<_>>();
        assembly(&mut subs, at_time, user, conn);
        parts.append(&mut subs)
    }
}

impl NewPart {
    fn create (self, user: &User, conn: &AppConn) -> TbResult<PartId> {
        use schema::parts::dsl::*;

        user.check_owner(self.owner, format!("user {} cannot create this part", user.get_id()))?;

        let values = (
            owner.eq(self.owner),
            what.eq(self.what),
            name.eq(self.name),
            vendor.eq(self.vendor),
            model.eq(self.model),
            purchase.eq(Utc::now()),
            time.eq(0),
            distance.eq(0),
            climb.eq(0),
            descend.eq(0),
            count.eq(0),
        );

        let part: Part = diesel::insert_into(parts).values(values).get_result(conn)?;
        Ok(part.id)
    }
}

#[get("/<part>")]
fn get (part: i32, user: &User, conn: AppDbConn) -> ApiResult<Part> {
    Ok(Json(PartId(part).part(user, &conn)?))
}

#[post("/", data="<newpart>")]
fn post(newpart: Json<NewPart>, user: &User, conn: AppDbConn) 
            -> Result<status::Created<Json<PartId>>, ApiError> {
    let id = newpart.clone().create(user, &conn)?;
    let url = uri! (get: i32::from(id));
    Ok (status::Created(url.to_string(), Some(Json(id))))
} 

#[get("/<part>/subparts?<time>")]
fn get_subparts (part: i32, time: Option<String>, user: &User, conn: AppDbConn) -> ApiResult<PartList> {
    Ok(Json(PartId(part).part(user, &conn)?.subparts(parse_time(time).unwrap_or_else(Utc::now), &conn)))
}

#[get("/<part>?assembly&<time>")]
fn get_assembly (part: i32, time: Option<String>, user: &User, conn: AppDbConn) -> ApiResult<Vec<(PartTypeId, Part)>> {
    let part = PartId::part(part.into(), user, &conn)?;
    let what = part.what;
    let mut res = vec!((what, part));
    assembly(&mut res, parse_time(time).unwrap_or_else(Utc::now), user, &conn);
    Ok(Json(res))
}

#[get("/mygear")]
fn mygear(user: &User, conn: AppDbConn) -> ApiResult<PartList> {    
    tbapi(Part::parts_by_user(user, true, &conn))
}

#[get("/myspares")]
fn myspares(user: &User, conn: AppDbConn) -> ApiResult<PartList> {    
    tbapi(Part::parts_by_user(user, false, &conn))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![get, post, get_subparts, get_assembly, mygear, myspares]
}