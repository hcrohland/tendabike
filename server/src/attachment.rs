//! The list of all historical attachments. This is the central piece of TendaBike.
//!
//! Attachments can be hierarchical
//! They are identified by part_id and attached time
//! If detached is none the part is still attached

use rocket_contrib::json::Json;

use self::schema::{attachments, parts};
use crate::user::*;
use crate::*;

use part::Part;

use diesel::{self, QueryDsl, RunQueryDsl};

/// Timeline of attachments
///
/// Every attachment of a part to a specified hook on a gear is an entry
/// Start and end time are noted
///
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Associations,
    Insertable,
    AsChangeset,
)]
#[primary_key(part_id, attached)]
#[changeset_options(treat_none_as_null = "true")]
#[belongs_to(PartType, foreign_key = "hook")]
// #[belongs_to(Part, foreign_key = "hook_id")]
pub struct Attachment {
    /// the sub-part, which is attached to the hook
    part_id: PartId,
    /// when it was attached
    attached: DateTime<Utc>,
    /// The gear the part is attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
    /// when it was removed again, "none" means "still attached"
    detached: Option<DateTime<Utc>>,
    /// usage count
    #[serde(default)]
    pub count: i32,
    /// usage time
    #[serde(default)]
    pub time: i32,
    /// Usage distance
    #[serde(default)]
    pub distance: i32,
    /// Overall climbing
    #[serde(default)]
    pub climb: i32,
    /// Overall descending
    #[serde(default)]
    pub descend: i32,
}

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct AttachmentDetail {
    #[serde(flatten)]
    a: Attachment,
    name: String,
    what: PartTypeId,
}

fn assembly(
    part: Part,
    at_time: DateTime<Utc>,
    conn: &AppConn,
) -> TbResult<Vec<(Part, Attachment)>> {
    use schema::attachments::dsl::*;

    let main = attached_to(part.id, at_time, &conn);
    let types = part.what.subtypes(conn);

    Ok(Attachment::belonging_to(&types)
        .inner_join(parts::table.on(parts::id.eq(part_id)))
        .filter(gear.eq(main))
        .filter(attached.lt(at_time))
        .filter(detached.is_null().or(detached.ge(at_time)))
        .order(parts::what)
        .order(hook)
        .select((schema::parts::all_columns, schema::attachments::all_columns)) // return only the Parts
        .load::<(Part, Attachment)>(conn)?)
}

/// Return the gear the part was attached to at at_time
pub fn attached_to(part: PartId, at_time: DateTime<Utc>, conn: &AppConn) -> PartId {
    use schema::attachments::dsl::*;

    let atts = attachments
        .filter(part_id.eq(part))
        .filter(attached.lt(at_time))
        .filter(detached.is_null().or(detached.ge(at_time)))
        .get_results::<Attachment>(conn)
        .expect("Error reading attachments");

    match atts.len() {
        0 => part,
        1 => atts[0].gear,
        _ => panic!(format!("multiple attaches {:?}", atts)),
    }
}

/// is detached a less than b?
///
/// none means indefinitely in the future
fn lt_detached(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> bool {
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

/* /// Return the minimum of two detached variables
fn min_detached(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    if lt_detached(a, b) {
        a
    } else {
        b
    }
}
 */
#[test]
fn test_detached() {
    let b = Some(Utc.ymd(2014, 7, 8).and_hms(9, 10, 11)); // `2014-07-08T09:10:11Z`
    let c = Some(Utc.ymd(2014, 7, 8).and_hms(9, 10, 10)); // `2014-07-08T09:10:10Z`
    assert!(lt_detached(c, b));
    assert!(!lt_detached(b, c));
    assert!(!lt_detached(b, b));
    assert!(!lt_detached(None, c));
    assert!(lt_detached(b, None));

    /*    assert_eq!(min_detached(c, b),c);
     assert_eq!(min_detached(b, c),c);
     assert_eq!(min_detached(c, c),c);
     assert_eq!(min_detached(c, None),c);
     assert_eq!(min_detached(None, c),c);
     assert_eq!(min_detached(None, None), None);
    */
}

impl Attachment {
    /// add the given usage to the attachement
    fn apply_usage(&mut self, usage: &Usage, conn: &AppConn) -> TbResult<Part> {
        debug!("Applying usage {:?}",usage);
        debug!("to attachment {:?}", self);
        self.count += usage.count;
        self.time += usage.time;
        self.distance += usage.distance;
        self.climb += usage.climb;
        self.descend += usage.descend;
        self.part_id.apply(usage, conn)
    }

    /// return the usage of the attachment
    fn usage(&self, factor: Factor, conn: &AppConn) -> Usage {
        Activity::find(self.gear, self.attached, self.detached, conn)
            .into_iter()
            .fold(Usage::none(self.attached), |acc, x| {
                acc.add_activity(&x, factor)
            })
    }

    /// 
    fn try_merge(&mut self, pred: &Self) -> bool {
        if self.part_id != pred.part_id || self.hook != pred.hook {
            return false;
        }
        debug!("merging {:?}", self);
        debug!("and {:?}", pred);
        self.attached = min(self.attached, pred.attached);
        self.detached = match (self.detached, pred.detached) {
            (None, _) | (_, None) =>  None,
            (Some(s), Some(p)) =>  Some(max(s,p))
        };
        debug!("to {:#?}", self);
        true
    }

    /// creates a new attachment with its side-effects
    ///
    /// - recalculates the usage counters in the attached assembly
    /// - persists everything into the database
    ///  -returns all affected parts or MyError::Conflict on collisions
    fn create(mut self, user: &dyn Person, conn: &AppConn) -> TbResult<Summary> {
        ensure!(!lt_detached(self.detached,Some(self.attached)), 
                Error::BadRequest(format!(" detached < attached: {:?}", self)));
        conn.transaction(|| {
            let mut attachments = self.collisions(conn)?;
            attachments.append(&mut read(
                self.part_id,
                Some(self.attached),
                self.detached,
                conn,
            )?);
            attachments.sort_by_key(|a|  a.attached);
            // self collisions and self.read do find both own attachment to this hook
            attachments.dedup();

            // if there is an exiting attachment, which started earlier and is not yet detached we detach it automatically
            let mut res = Summary::default();
            for mut pred in attachments.into_iter() {
                if lt_detached(self.detached, Some(pred.attached)) {
                    continue;
                }

                if self.try_merge(&pred) {
                    // extend attachment
                    debug!("merging predecessor");
                    res.append(&mut pred.delete(conn)?);
                } else if pred.attached <= self.attached {
                    // predecessor gets detached
                    debug!("detaching predecessor");
                    pred.detached = Some(self.attached);
                    res.append(&mut pred.patch(user, conn)?);
                } else if self.detached.is_none() && pred.attached > self.attached {
                    // this attachment ends
                    debug!("Adjusting detach time");
                    self.detached = Some(pred.attached);
                } else {
                    return Err(
                        Error::Conflict(format!("{:?} collides with {:?}", self, pred)).into(),
                    );
                }
            }

            let usage = self.usage(Factor::Add, conn);
            let part = self.apply_usage(&usage, conn)?;
            let att = diesel::insert_into(attachments::table) // Store the attachment in the database
                .values(self)
                .get_result::<Attachment>(conn)
                .context("Could not insert attachment")?;
            res.attachments.push(att.add_details(part.name.clone(), part.what));
            res.parts.push(part); // and register changes
            Ok(res)
        })
    }

    /// deletes an attachment with its side-effects
    ///
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    fn delete(self, conn: &AppConn) -> TbResult<Summary> {
        conn.transaction(|| {
            let ctx = format!("Could not delete attachment {:#?}", self);
            let mut att = diesel::delete(attachments::table.find((self.part_id, self.attached))) // delete the attachment in the database
                .get_result::<Attachment>(conn)
                .context(ctx)?;
            
            let usage = att.usage(Factor::Sub, conn);
            let part = att.apply_usage(&usage,conn)?;
            att.detached = Some(att.attached);
            return Ok(Summary{
                attachments: vec![att.add_details("".into(), 0.into())],
                parts: vec![part],
                activities: vec![]
            })
        })
    }

    /// change an attachment identified by part_id and attached
    ///
    /// This is the main function to manage attachments
    /// - if the attachment does not exist, create the database object
    /// - if detached <= attached delete the attachment
    /// - if detached changed, change the database object
    ///
    /// returns
    /// - MyError::Conflict if the hook_id does not match
    /// - all recalculated parts on success
    fn patch(mut self, user: &dyn Person, conn: &AppConn) -> TbResult<Summary> {
        self.part_id.checkuser(user, conn)?;
        conn.transaction(|| {
            let mut state = match attachments::table
                .find((self.part_id, self.attached))
                .filter(attachments::gear.eq(self.gear))
                .for_update()
                .get_result::<Attachment>(conn)
            {
                Err(diesel::result::Error::NotFound) => return self.create(user, conn),
                Err(e) => return Err(e.into()),
                Ok(x) => x,
            };
            
            ensure!(
                state.hook == self.hook,
                Error::Conflict(format!(
                    "part {} already attached to hook {}, instead of {}",
                    self.part_id, state.hook, self.hook
                ))
            );

            ensure!(
                self.detached != state.detached, // No change!
                Error::BadRequest(String::from("Attachment already exists"))
            );

            if let Some(detached) = self.detached {
                if detached <= state.attached {
                    return self.delete(conn)
                }
            }

            let factor;
            if lt_detached(self.detached, state.detached) {
                state.attached = self.detached.unwrap();
                factor = Factor::Sub
            } else {
                state.attached = state.detached.unwrap();
                state.detached = self.detached;
                let coll = state.collisions(conn)?;
                ensure!(
                    coll.is_empty(),
                    Error::BadRequest(format!("Attachment collision with {:?}", coll))
                );
                factor = Factor::Add
            };

            let usage = state.usage(factor, conn);
            let part = self.apply_usage(&usage, conn)?; // and register changes
            let attachment = AttachmentDetail {
                    a: self.save_changes::<Attachment>(conn)?,
                    name: part.name.clone(), 
                    what: part.what
                };
            Ok(Summary {
                parts: vec![part], 
                attachments: vec![attachment],
                ..Default::default()
            })
        })
    }

    /// find other parts which are attached to the same hook as myself in the given timeframe
    ///
    /// part_id is actually ignored
    /// returns the full attachments for these parts.
    fn collisions(&self, conn: &AppConn) -> TbResult<Vec<Attachment>> {
        let what= self.part_id.what(conn)?;
        let mut query = attachments::table
            .inner_join(
                parts::table.on(parts::id
                    .eq(attachments::part_id) // join corresponding part
                    .and(parts::what.eq(what))),
            ) // where the part has my type
            .filter(attachments::gear.eq(self.gear))
            .filter(attachments::hook.eq(self.hook))
            .filter(attachments::detached.is_null().or(attachments::detached.gt(self.attached)),)       
            .into_boxed();
        if let Some(detached) = self.detached {
            query = query.filter(attachments::attached.lt(detached));
        }
        Ok(query
            .select(schema::attachments::all_columns) // return only the attachment
            .order(attachments::attached)
            .load::<Attachment>(conn)?)
    }

    fn add_details(self, name: String, what: PartTypeId) -> AttachmentDetail {
        AttachmentDetail {
            name,
            what,
            a: self
        }
    }

    fn enrich(self, conn: &AppConn) -> TbResult<AttachmentDetail> {
        use schema::parts::dsl::{parts,name,what};
        
        let (n, w) = parts
            .find(self.part_id)
            .select((name,what))
            .get_result::<(String,PartTypeId)>(conn)?;
        Ok(self.add_details(n, w))
    }
}

pub fn register(act: &Activity, usage: &Usage, conn: &AppConn) -> Vec<AttachmentDetail> {
    use schema::attachments::dsl::*;

    if let Some(act_gear) = act.gear {
        diesel::update(
            attachments
                .filter(gear.eq(act_gear))
                .filter(attached.lt(act.start))
                .filter(detached.is_null().or(detached.ge(act.start)))
        )
        .set((
            time.eq(time + usage.time),
            climb.eq(climb + usage.climb),
            descend.eq(descend + usage.descend),
            distance.eq(distance + usage.distance),
            count.eq(count + usage.count),
        ))
        .get_results::<Attachment>(conn).expect("Database Error")
        .into_iter()
        .map(|a| a.enrich(conn).expect("couldn't enrich attachment"))
        .collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

fn rescan_activities(conn: &AppConn) {
    use schema::attachments::dsl::*;

    conn.transaction::<_,anyhow::Error,_> (|| {
        diesel::update(attachments)
        .set((
            time.eq(0),
            climb.eq(0),
            descend.eq(0),
            distance.eq(0),
            count.eq(0),
        )).execute(conn)?;

        let acts = schema::activities::table.get_results::<Activity>(conn).expect("Could not read activities");

        for act in acts {
            register(&act, &act.usage(Factor::Add), conn);
        }
        Ok(())
    }).expect("Transaction failed");
}

pub fn for_parts(partlist: Vec<Part>, conn: &AppConn) -> TbResult<Summary> {
    use schema::attachments::dsl::*;
    use schema::parts::dsl::{parts,id,name,what};
    let ids: Vec<_> = partlist.iter().map(|p| p.id).collect();
    let atts = attachments
        .filter(part_id.eq_any(ids.clone()))
        .or_filter(gear.eq_any(ids))
        .inner_join(parts.on(id.eq(part_id)))
        .select((schema::attachments::all_columns,name,what))
        .get_results::<AttachmentDetail>(conn)?;

    Ok(Summary {
        parts: partlist,
        attachments: atts,
        ..Default::default()
    })
}

pub fn parts_per_activity(act: &Activity, conn: &AppConn) -> Vec<PartId> {
    use schema::attachments::dsl::*;

    let mut res = Vec::new();
    if let Some(act_gear) = act.gear {
        res.push(act_gear); // We need the gear too!
        res.append(
            &mut attachments
                .filter(gear.eq(act_gear))
                .filter(attached.lt(act.start))
                .filter(detached.is_null().or(detached.ge(act.start)))
                .select(part_id)
                .get_results::<PartId>(conn)
                .expect("Error reading attachments"),
        );
    }
    res
}

#[patch("/", data = "<attachment>")]
fn patch(attachment: Json<Attachment>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(attachment.patch(user, &conn))
}

/// Where was this part attached in the given time frame?
///
#[get("/<part_id>?<start>&<end>")]
fn get(
    part_id: i32,
    start: Option<String>,
    end: Option<String>,
    user: &User,
    conn: AppDbConn,
) -> ApiResult<Vec<(Attachment, String)>> {
    let mut res = Vec::new();
    let part = PartId::get(part_id, user, &conn)?;
    let start = parse_time(start)?;
    let end = parse_time(end)?;

    for a in read(part, start, end, &conn)? {
        res.push((a, a.gear.name(&conn)?));
    }
    Ok(Json(res))
}

#[get("/assembly/<part>?<time>")]
fn get_assembly(
    part: i32,
    time: Option<String>,
    user: &User,
    conn: AppDbConn,
) -> ApiResult<Vec<(Part, Attachment)>> {
    let part = PartId::part(part.into(), user, &conn)?;
    let time = parse_time(time)?.unwrap_or_else(Utc::now);
    Ok(Json(assembly(part, time, &conn)?))
}

/// Return all attachment for this part in the given time Frame
///
/// Start == None means from the beginning of time
fn read(
    part: PartId,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    conn: &AppConn,
) -> TbResult<Vec<Attachment>> {
    let mut atts = attachments::table
        .order(attachments::attached) // Ordered by time
        .filter(attachments::part_id.eq(part)) // is the right part
        .for_update() // cannot be boxed!
        .get_results::<Attachment>(conn)?;

    if let Some(end) = end {
        atts.retain(|&a| a.attached < end); // attached before end
    }
    if let Some(start) = start {
        atts.retain(|&a| a.detached.is_none() || a.detached.unwrap() > start); // detached after start
    }

    Ok(atts)
}

use rocket::http::Status;

#[get("/rescan")]
fn rescan(
    _user: Admin,
    conn: AppDbConn,
) -> Status {
    rescan_activities(&conn);
    Status::Ok
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, patch, get_assembly, rescan]
}