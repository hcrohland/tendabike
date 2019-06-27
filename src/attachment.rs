use std::cmp::{max, min};

use chrono::{
    Utc,
    Local,
    DateTime,
    TimeZone
};

use rocket_contrib::json::Json;
use rocket::response::status;

use self::schema::{parts, attachments};
use crate::user::*;
use crate::*;

use part::Assembly;

use self::diesel::prelude::*;

use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};



/// Timeline of attachments
/// 
/// Every attachement of a part to another part (hook) is an entry
/// Start and end time are noted
/// 
#[derive(Clone, Copy, Debug, PartialEq, 
        Serialize, Deserialize, 
        Queryable, Identifiable, Associations, Insertable, AsChangeset)]
#[primary_key(part_id, attached)]
// #[belongs_to(Part, foreign_key = "hook_id")]
struct Attachment {
    // the sub-part, which is attached to the hook
    part_id: PartId,
    // the hook, to which part_id is attached
    hook_id: PartId,
    // when it was attached
    attached: DateTime<Utc>,
    // when it was removed again
    detached: Option<DateTime<Utc>>,
}

pub fn subparts(part: PartId, at_time: DateTime<Utc>, conn: &AppConn) -> Vec<PartId> {
    use schema::attachments::dsl::*;

    attachments.select(part_id)
        .filter(hook_id.eq(part))
        .filter(attached.lt(at_time)).filter(detached.ge(at_time))
        .load::<PartId>(conn).expect("could not read attachments")
}

/// is the part attached to a hook?
pub fn is_attached(part: PartId, at_time: DateTime<Utc>, conn: &AppConn) -> bool {
    use schema::attachments::dsl::*;

    match attachments.count()
        .filter(part_id.eq(part))
        .filter(attached.lt(at_time)).filter(detached.ge(at_time))
        .get_result(conn).expect("could not read attachments"){
            0 => false,
            1 => true,
            _ => panic!("could not read attachments"),
        }
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

        let mut query = attachments.filter(part_id.eq(self.hook_id)).into_boxed(); // We are looking for parents of hook_id!
        if let Some(x) = self.detached { query = query.filter(attached.le(x)) } 
        query.filter(detached.is_null().or(detached.gt(self.attached))) // anything in our timeframe matches
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

    /// creates a new attachment with its side-effects
    /// 
    /// - recalculates the usage counters in the attached assembly
    /// - persists everything into the database
    ///  -returns all affected parts
    /// 
    /// does not check for collisions (yet?)
    fn create (&self, conn: &AppConn) -> TbResult<Assembly> {
        // let siblings = self.siblings(&att);

        let att = self.safe(conn)?;

        let tops = att.ancestors(conn);  // we need the gear, but also get potential spare parts

        let mut res = Assembly::new();
        for top in tops {
            let acts = Activity::find(top.hook_id, top.attached, top.detached, conn);
            for act in acts {
                self.part_id.traverse(&mut res, &act.usage(), 1, conn)?
                // siblings.pick(&usage).traverse(&mut map, &usage, -1, conn)
            }
        }
        Ok(res)
    }

    /// find other parts which are attached to the same hook as myself in the given timeframe
    /// 
    /// returns the full attachments for these parts.
    fn siblings(&self, what: PartTypeId, conn: &AppConn) -> Vec<Attachment> {
        let mut query  = attachments::table
                .inner_join(parts::table.on(parts::id.eq(attachments::part_id) // join corresponding part
                                            .and(parts::what.eq(what))))  // where the part has my type
                .filter(attachments::hook_id.eq(self.hook_id)).into_boxed(); // ... and is hooked to the parent
        if let Some(detached) = self.detached { query = query.filter(attachments::attached.lt(detached)) }
        query.filter(attachments::detached.is_null().or(attachments::detached.ge(self.attached))) // ... and in the given time frame
                .select(schema::attachments::all_columns) // return only the attachment
                .load::<Attachment>(conn).expect("could not read attachments")
    }
}

#[post("/", data="<attachment>")]
fn attach (attachment: Json<Attachment>, _user: User, conn: AppDbConn) 
            -> TbResult<status::Created<Json<Assembly>>> {
    Ok(conn.test_transaction::<_,MyError,_> (||{
        let res = attachment.create(&conn)?;
        let url = format!("/attach/{}/{}", attachment.part_id, attachment.attached);
        Ok(status::Created(url, Some(Json(res))))
    }))    
} 

#[get("/<part_id>/<hook_id>/<start>?<end>")]
fn top (part_id: i32, hook_id: i32, start: String, end: Option<String>, user: User, conn: AppDbConn) -> TbResult<Json<Vec<Attachment>>> {
    let att = Attachment {
        attached: Local.datetime_from_str(&start, "%FT%T").expect("no start").with_timezone(&Utc),
        detached: end.map(|x| Local.datetime_from_str(&x, "%FT%T").expect("no end").with_timezone(&Utc)),
        part_id: part_id.into(),
        hook_id: hook_id.into(),
    };
    let what = PartId::read(part_id, &user, &conn)?.what;

    Ok(Json(att.siblings(what, &conn)))
}


pub fn routes () -> Vec<rocket::Route> {
    routes![top, attach]
}