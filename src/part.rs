use std::cmp::{max, min};
use std::collections::HashMap;

use chrono::{
    Utc,
    Local,
    DateTime,
    TimeZone
};

use rocket_contrib::json::Json;
use rocket::response::status;

use self::schema::{parts, part_types, attachments};
use crate::user::*;
use crate::*;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

pub type PartType = i32;

/// List of of all valid part types.
/// 
/// We distingish main parts from spares:
/// - Main parts can be used for an activity - like a bike
/// - Spares can be attached to other parts and are subparts of main parts
#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Identifiable, Associations, PartialEq)]
#[table_name = "part_types"]
pub struct PartTypes {
    /// The primary key
    pub id: PartType,
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
    pub what: PartType,
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
    /// 
    /// Assumes authorization checked
    fn read (self, conn: &AppConn) -> TbResult<Part> {
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
    fn apply (self, usage: &Usage, factor: i32, conn: &AppConn) -> TbResult<Part> {
        use schema::parts::dsl::*;

        if factor != 0 {
            info!("Applying usage to part {}", self);
            Ok(diesel::update(parts.find(self))
                .set((  time.eq(time + usage.time * factor),
                        climb.eq(climb + usage.climb * factor),
                        descend.eq(descend + usage.descend * factor),
                        distance.eq(distance + usage.distance * factor),
                        count.eq(count + usage.count * factor)))
                .get_result::<Part>(conn)?)
        } else {
            Ok (parts.find(self).first::<Part>(conn)?)
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

    /// account for usage of the assembly attached to self 
    /// 
    /// returns all parts affected
    /// 
    /// if the usage is Usage::none() it simply returns the assembly
    /// - It should not update the database in this case, but does for now.
    fn traverse (self, map: & mut Assembly, usage: &Usage, factor: i32, conn: &AppConn) -> TbResult<()> {
        self.subparts(usage.start, conn)
                .into_iter().map(|x| x.traverse(map, usage, factor, conn))
                .for_each(drop);

        map.insert(self, self.apply(usage, factor, conn)?);
        Ok(())
    }

    /// account for usage of the assembly attached to self 
    /// 
    /// returns all parts affected
    /// checks if the user is authorized
    pub fn utilize (self, map: & mut Assembly, usage: Usage, user: &dyn Person, conn: &AppConn) -> TbResult<()> {
        self.checkuser(user, conn)?.traverse(map, &usage, 1, conn)
    }
}

/// Timeline of attachments
/// 
/// Every attachement of a part to another part (hook) is an entry
/// Start and end time are noted
/// 
#[derive(Clone, Copy, Debug, PartialEq, 
        Serialize, Deserialize, 
        Queryable, Identifiable, Associations, Insertable, AsChangeset)]
#[primary_key(part_id, attached)]
#[belongs_to(Part, foreign_key = "hook_id")]
struct Attachment {
    // the sub-part, which is attached to the hook
    pub part_id: PartId,
    // the hook, to which part_id is attached
    pub hook_id: PartId,
    // when it was attached
    pub attached: DateTime<Utc>,
    // when it was removed again
    pub detached: DateTime<Utc>,
}

impl Attachment {

    /// Store the attachment in the database
    fn safe (&self, conn: &AppConn) -> TbResult<Attachment> {
        Ok(diesel::insert_into(attachments::table).values(self).get_result(conn)?)
    }

    /// Retrieve the attachments for self.hook_id in the timeframe defined by self
    /// 
    /// self.part_id is ignored!
    /// parents are found in hook_id of the resulting attachments
    /// panics on unexpected database behaviour
    fn parents(&self, conn: &AppConn) -> Vec<Attachment> {
        use schema::attachments::dsl::*;

        attachments
            .filter(part_id.eq(self.hook_id)) // We are looking for parents of hook_id!
            .filter(attached.le(self.detached)).filter(detached.gt(self.attached)) // anything in our timeframe matches
            .load(conn).expect("Could not read attachments")
    }

    /// retrieve the topmost part for self->hook_id
    /// 
    /// when explicitly called self.part_id should be set to self.hook_id
    /// panics on unexpected database behaviour
    fn ancestors (self, conn: &AppConn) -> Vec<Attachment> {
        let mut res = Vec::new();
        
        let parents = self.parents(conn);

        // if there are no parents, it is topmost
        // but we ignore the starting point if it has part_id == hook_id.
        // all other results will have them differing
        if parents.is_empty() && self.part_id != self.hook_id { 
            res.push(self); 
        } else {
            for mut p in parents {
                // We only want to have the intersection of the parent and the given window!
                p.attached = max(p.attached, self.attached);
                p.detached = min(p.detached, self.detached);
                res.append(&mut p.ancestors(conn))
            }
        }
        res
    }

    fn create (&self, conn: &AppConn) -> TbResult<Assembly> {
        // let siblings = self.siblings(&att);

        let att = self.safe(conn)?;

        let tops = att.ancestors(conn);
        let mut map = Assembly::new();

        for top in tops {
            let uses = Activity::find(top.hook_id, top.attached, top.detached, conn);
            for usage in uses {
                self.part_id.traverse(&mut map, &usage, 1, conn)?
                // siblings.pick(&usage).traverse(&mut map, &usage, -1, conn)
            }
        }
        Ok(map)
    }

    /// find other parts which are attached to the same hook as myself in the given timeframe
    /// 
    /// returns the full attachments for these parts.
    fn siblings(&self, what: PartType, conn: &AppConn) -> Vec<Attachment> {
        attachments::table
                .inner_join(parts::table.on(parts::id.eq(attachments::part_id) // join corresponding part
                                            .and(parts::what.eq(what))))  // where the part has my type
                .filter(attachments::hook_id.eq(self.hook_id)) // ... and is hooked to the parent
                .filter(attachments::attached.lt(self.detached).and(attachments::detached.ge(self.attached))) // ... and in the given time frame
                .select(schema::attachments::all_columns) // return only the attachment
                .load::<Attachment>(conn).expect("could not read attachments")
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
                x.attachment(x.id, Utc::now(), Utc::now()).parents(conn).is_empty() // only parts which are not attached
            }).collect())
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
        let mains: HashSet<i32> = part_types::table.select(part_types::id).filter(part_types::main.eq(true))
            .load::<i32>(conn).expect("error loading PartTypes").into_iter().collect();

        // only return the main parts
        Ok(part_list.into_iter()
            .filter(|x| mains.contains(&x.what)).map(|x| x.id)
            .collect())
    }

    /// Instantiate a new Attachment object for Part self
    /// 
    /// This is not persisted in the database, but can be used for a lot of operations
    fn attachment<P> (&self, hook: P, start: DateTime<Utc>, end: DateTime<Utc>) -> Attachment
        where P: Into<PartId>
    {
        Attachment {
            part_id: self.id,
            hook_id: hook.into(),
            attached: start,
            detached: end 
        }
    }
}

#[get("/types")]
fn types(_user: User, conn: AppDbConn) -> Json<Vec<PartTypes>> {
    Json(Part::types(&conn))
}

#[get("/<part>")]
fn get (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Part>> {
    PartId::get(part, &user, &conn)?.read(&conn).map (Json)
}

#[post("/attach", data="<attachment>")]
fn attach (attachment: Json<Attachment>, _user: User, conn: AppDbConn) 
            -> TbResult<status::Created<Json<Assembly>>> {
    Ok(conn.test_transaction::<_,MyError,_> (||{
        let res = attachment.create(&conn)?;
        let url = format!("/attach/{}/{}", attachment.part_id, attachment.attached);
        Ok(status::Created(url, Some(Json(res))))
    }))    
} 

#[get("/top/<part>/<hook>/<start>/<end>")]
fn top (part: i32, hook: i32, start: String, end: String, user: User, conn: AppDbConn) -> TbResult<Json<Vec<Attachment>>> {
    let start = Local.datetime_from_str(&start, "%FT%T").expect("no start").with_timezone(&Utc);
    let end = Local.datetime_from_str(&end, "%FT%T").expect("no end").with_timezone(&Utc);

    let part = PartId::get(part, &user, &conn)?.read(&conn)?;
    Ok(Json(        
            part.attachment(hook, start, end).siblings(part.what, &conn)
    ))
}

#[get("/<part>?assembly")]
fn get_assembly (part: i32, user: User, conn: AppDbConn) -> TbResult<Json<Assembly>> {
    let mut map = Assembly::new();

    PartId::get(part, &user, &conn)?.traverse(&mut map, &Usage::none(), 0, &conn)?;
    
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
    routes![types, get, get_assembly, mygear, myspares, attach, top]
}