use std::cmp::max;

use chrono::{
    Utc,
    Local,
    DateTime,
    TimeZone
};

use rocket_contrib::json::Json;

use self::schema::{parts, attachments};
use crate::user::*;
use crate::*;

use part::Assembly;

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
#[changeset_options(treat_none_as_null = "true")]
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
        .filter(attached.lt(at_time)).filter(detached.is_null().or(detached.ge(at_time)))
        .load::<PartId>(conn).expect("could not read attachments")
}

/// is the part attached to a hook?
pub fn is_attached(part: PartId, at_time: DateTime<Utc>, conn: &AppConn) -> bool {
    use schema::attachments::dsl::*;

    match attachments.count()
        .filter(part_id.eq(part))
        .filter(attached.lt(at_time)).filter(detached.is_null().or(detached.ge(at_time)))
        .get_result(conn).expect("could not read attachments"){
            0 => false,
            1 => true,
            _ => panic!("could not read attachments"),
        }
}

fn lt_detached (a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> bool {
    if let Some(a) = a {
        if let Some(b) = b {
            a < b
        } else {
            true
        }
    } else {
        false
    } 
}

fn min_detached(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    if lt_detached(a, b) {
        a
    } else {
        b
    }
}

#[test]
fn test_detached (){
    let b = Some(Utc.ymd(2014, 7, 8).and_hms(9, 10, 11)); // `2014-07-08T09:10:11Z`
    let c = Some(Utc.ymd(2014, 7, 8).and_hms(9, 10, 10)); // `2014-07-08T09:10:10Z`
    assert!(lt_detached(c,b));
    assert!(!lt_detached(b,c));
    assert!(!lt_detached(b,b));
    assert!(!lt_detached(None,c));
    assert!(lt_detached(b,None));

    assert_eq!(min_detached(c, b),c);
    assert_eq!(min_detached(b, c),c);
    assert_eq!(min_detached(c, c),c);
    assert_eq!(min_detached(c, None),c);
    assert_eq!(min_detached(None, c),c);
    assert_eq!(min_detached(None, None), None);
    
}

impl Attachment {
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
    /// It only returns the timeframe of self. Not the full attachments.
    /// panics on unexpected database behaviour
    fn ancestors (self, conn: &AppConn) -> Vec<Attachment> {
        let mut res = Vec::new();
        
        let parents = dbg!(self.parents(conn));

        // if there are no parents, it is topmost
        // but we ignore the starting point if it has part_id == hook_id.
        // all other results will have them differing
        if parents.is_empty() && self.part_id != self.hook_id { 
            res.push(self); 
        } else {
            for mut p in parents {
                // We only want to have the intersection of the parent and the given window!
                // This is important, since ancestors might live longer than childs and 
                p.attached = max(p.attached, self.attached);
                p.detached = min_detached(p.detached, self.detached);
                
                res.append(&mut p.ancestors(conn))
            }
        }
        res
    }

    fn register (&self, factor: Factor, conn: &AppConn) -> TbResult<Assembly> {

        let tops = dbg!(self).ancestors(conn);  // we need the gear, but also get potential spare parts

        let mut res = Assembly::new();
        for top in tops {
            let acts = Activity::find(top.hook_id, top.attached, top.detached, conn);
            for act in acts {
                self.part_id.traverse(&mut res, &act.usage(), factor, conn)?
            }
        }
        Ok(res)
    }

    /// creates a new attachment with its side-effects
    /// 
    /// - recalculates the usage counters in the attached assembly
    /// - persists everything into the database
    ///  -returns all affected parts
    /// 
    /// does not check for collisions (yet?)
    fn create (&self, user: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction (||{
            if !self.siblings(user,conn)?.is_empty() {
                return Err(MyError::BadRequest("".into()));
            }
            diesel::insert_into(attachments::table) // Store the attachment in the database
                    .values(self).get_result::<Attachment>(conn)?
                    .register(Factor::Add, conn)              // and register changes
        })
    }

    fn delete (self, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction (||{
            diesel::delete(attachments::table.find((self.part_id, self.attached))) // delete the attachment in the database
                    .get_result::<Attachment>(conn)?
                    .register(Factor::Sub, conn)              // and register changes
        })

    }

    fn patch (self, user: &dyn Person, conn: &AppConn) -> TbResult<Assembly> {
        conn.transaction (||{
            let mut state = 
            match attachments::table.find((self.part_id, self.attached))
                            .for_update().get_result::<Attachment>(conn) {
                Err(diesel::result::Error::NotFound) => return self.create(user, conn),
                Err(e) => return Err(e.into()),
                Ok(x) => x,
            };
            if state.hook_id != self.hook_id {
                return Err(MyError::NotFound(format!("part {} not attached to hook {}", self.part_id, self.hook_id)));
            }

            if self.detached == state.detached { // No change!
                return Ok(Assembly::new());
            }

            if let Some(detached) = self.detached {
                if detached <= state.attached { // 
                    return self.delete(conn);
                }
            }

            let factor = if lt_detached(self.detached, state.detached) {
                state.attached = self.detached.unwrap();
                Factor::Sub
            } else {
                state.attached = state.detached.unwrap();
                state.detached = self.detached;
                Factor::Add
            };
            
            self.save_changes::<Attachment>(conn)?;
            state.register(factor, conn)              // and register changes
        })

    }
 
    /// find other parts which are attached to the same hook as myself in the given timeframe
    /// 
    /// returns the full attachments for these parts.
    fn siblings(&self, user: &dyn Person, conn: &AppConn) -> TbResult<Vec<Attachment>> {
        let what = PartId::read(self.part_id.into(), user, conn)?.what;
        let mut query  = attachments::table
                .inner_join(parts::table.on(parts::id.eq(attachments::part_id) // join corresponding part
                                            .and(parts::what.eq(what))))  // where the part has my type
                .filter(attachments::hook_id.eq(self.hook_id)).into_boxed(); // ... and is hooked to the parent
        if let Some(detached) = self.detached { query = query.filter(attachments::attached.lt(detached)) }
        Ok(query.filter(attachments::detached.is_null().or(attachments::detached.ge(self.attached))) // ... and in the given time frame
                .select(schema::attachments::all_columns) // return only the attachment
                .load::<Attachment>(conn)?)
    }
}

#[patch("/", data="<attachment>")]
fn patch(attachment: Json<Attachment>, user: User, conn: AppDbConn) 
            -> TbResult<Json<Assembly>> {
    Ok(Json(attachment.patch(&user, &conn)?))
} 

#[get("/<part_id>/<hook_id>/<start>?<end>")]
fn top (part_id: i32, hook_id: i32, start: String, end: Option<String>, user: User, conn: AppDbConn) -> TbResult<Json<Vec<Attachment>>> {
    let att = Attachment {
        attached: Local.datetime_from_str(&start, "%FT%T").expect("no start").with_timezone(&Utc),
        detached: end.map(|x| Local.datetime_from_str(&x, "%FT%T").expect("no end").with_timezone(&Utc)),
        part_id: part_id.into(),
        hook_id: hook_id.into(),
    };

    Ok(Json(att.siblings(&user, &conn)?))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![top, patch]
}
