//! The list of all historical attachments. This is the central piece of TendaBike.
//!
//! Attachments can be hierarchical
//! They are identified by part_id and attached time

use chrono::MAX_DATETIME;
use diesel::{self, QueryDsl, RunQueryDsl};
use rocket_contrib::json::Json;

use crate::*;
use schema::attachments;
/// Description of an Attach or Detach request
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Event {
    /// the part which should be change
    part_id: PartId,
    /// when it the change happens
    time: DateTime<Utc>,
    /// The gear the part is or will be attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
}

impl Event {
    /// End an attachment for 'self.part' and all it's childs
    ///
    /// Check's authorization and taht the part is attached
    ///
    fn detach(self, user: &dyn Person, conn: &AppConn) -> TbResult<Summary> {
        info!("detach {:?}", self);
        // check user
        self.part_id.checkuser(user, conn)?;
        conn.transaction(|| {
            let target = self
                .at(conn)?
                .ok_or(Error::NotFound("part not attached".into()))?;

            ensure!(
                self.hook == target.hook && self.gear == target.gear,
                Error::BadRequest(format!("{:?} does not match attachment", self))
            );

            self.detach_assembly(target, conn)
        })
    }

    /// detach the whole assembly pointed at by 'self'
    ///
    /// 'target' is the corresponding Attachment. handed in as an optimization
    /// it must be the same as self.at(conn)
    /// 
    /// When the 'self.partid' has child parts they are attached to that part
    ///
    fn detach_assembly(self, target: Attachment, conn: &AppConn) -> TbResult<Summary> {
        debug!("- detaching {}", target.part_id);
        let subs = self.assembly(target.gear, conn)?;
        let mut hash = SumHash::new(target.detach(self.time, conn)?);
        for sub in subs { 
            sub.shift(self.time, target.part_id, &mut hash, conn)?;
        }
        Ok(hash.collect())
    }

    /// Create an attachment for 'self.part' and all it's childs
    /// It will detach any part - and childs therof - which art attached already
    ///
    /// When the detached part has child parts they are attached to that part
    ///
    fn attach(self, user: &dyn Person, conn: &AppConn) -> TbResult<Summary> {
        info!("attach {:?}", self);
        // check user
        let part = self.part_id.part(user, conn)?;
        // and types
        let mytype = part.what.get(conn)?;
        ensure!(
            mytype.hooks.contains(&self.hook),
            Error::BadRequest(format!(
                "Type {} cannot be attached to hook {}",
                mytype.name, self.hook
            ))
        );
        let gear = self.gear.part(user, conn)?;
        ensure!(
            mytype.main == gear.what || mytype.hooks.contains(&gear.what),
            Error::BadRequest(format!(
                "Type {} cannot be attached to gear {}",
                mytype.name, gear.what
            ))
        );
        conn.transaction(|| {
            let mut hash = SumHash::default();

            // detach self assembly
            if let Some(target) = self.at(conn)? {
                info!("detaching self assembly");
                hash.merge(self.detach_assembly(target, conn)?);
            }

            // detach target assembly
            if let Some(att) = self.occupant(conn)? {
                info!("detaching target assembly {}", att.part_id);
                hash.merge(self.detach_assembly(att, conn)?);
            }

            let subs = self.assembly(self.part_id, conn)?;
            // reattach the assembly
            info!("attaching assembly");
            debug!("- attaching {}", self.part_id);
            let (sum, det) = self.attach_one(conn)?;
            hash.merge(sum);
            for att in subs {
                let sub_det = att.shift(self.time, self.gear, &mut hash, conn)?;
                if sub_det == det && det < att.detached {
                    trace!("reattaching {} to {} at {}", att.part_id, self.part_id, det);
                    let ev = Event {
                        part_id: att.part_id,
                        hook: att.hook,
                        gear: self.part_id,
                        time: det,
                    };
                    let (sum, _) = ev.attach_one(conn)?;
                    hash.merge(sum);
                } 
            }
            Ok(hash.collect())
        })
    }

    /// create Attachment for one part according to self
    ///
    /// * The part must not be attached somewhere at the event time
    /// * Also the hook must not be occupied at the event time
    /// * Detach time is adjusted according to later attachments
    ///
    /// If the part is attached already to the same hook, the attachments are merged
    fn attach_one(self, conn: &AppConn) -> TbResult<(Summary, DateTime<Utc>)> {
        let mut hash = SumHash::default();
        // when does the current attachment end
        let mut end = MAX_DATETIME; 
        // the time the current part will be detached
        // we need this to reattach subparts
        let mut det = MAX_DATETIME; 

        let what = self.part_id.what(conn)?;

        if let Some(next) = self.next(what, conn)? {
            trace!("successor at {}", next.attached);
            // something else is already attached to the hook
            // the new attachment ends when the next starts
            end = next.attached;
            det = next.attached;
        }

        if let Some(next) = self.after(conn)? {
            if end > next.attached { // is this attachment earlier than the previous one?
                if next.gear == self.gear && next.hook == self.hook {
                    trace!("still attached until {}", next.detached);
                    // the previous one is the real next so we keep 'det'!
                    // 'next' will be replaced by 'self' but 'end' is taken from 'next'
                    end = next.detached;
                    let sum = next.delete(conn)?;
                    hash.merge(sum);
                } else {
                    trace!("changing gear/hook from {}/{} to {}/{}", self.gear, self.hook, next.gear, next.hook);
                    // it is attached to a different hook later
                    // the new attachment ends when the next starts
                    end = next.attached;
                    det = next.attached
                }
            }
        }

        // try to merge previous attachment
        if let Some(prev) = self.adjacent(conn)? {
            trace!("adjacent starting {}", prev.attached);
            hash.merge(prev.detach(end, conn)?)
        } else {
            trace!("create {:?}\n", self);
            hash.merge(self.attachment(end).create(conn)?);
        }

        Ok((hash.collect(), det))
    }

    /// create an attachment of self with the given detached time
    fn attachment(self, detached: DateTime<Utc>) -> Attachment {
        Attachment {
            part_id: self.part_id,
            gear: self.gear,
            hook: self.hook,
            attached: self.time,
            detached,
            count: 0,
            time: 0,
            climb: 0,
            descend: 0,
            distance: 0,
        }
    }

    /// Return Attachment if another part is attached to same hook at Event
    fn occupant(&self, conn: &AppConn) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        use schema::parts;
        let what = self.part_id.what(conn)?;

        Ok(attachments
            .inner_join(
                parts::table.on(parts::id
                    .eq(part_id) // join corresponding part
                    .and(parts::what.eq(what))),
            ) // where the part has my type
            .filter(gear.eq(self.gear))
            .filter(hook.eq(self.hook))
            .select(schema::attachments::all_columns) // return only the attachment
            .filter(attached.le(self.time))
            .filter(detached.gt(self.time))
            .first::<Attachment>(conn)
            .optional()?)
    }

    /// Return Attachment if some other part is attached to same hook after the Event
    fn next(&self, what: PartTypeId, conn: &AppConn) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        use schema::parts;

        Ok(attachments
            .for_update()
            .inner_join(
                parts::table.on(parts::id
                    .eq(part_id) // join corresponding part
                    .and(parts::what.eq(what))),
            ) // where the part has my type
            .filter(gear.eq(self.gear))
            .filter(hook.eq(self.hook))
            .filter(part_id.ne(self.part_id))
            .select(schema::attachments::all_columns) // return only the attachment
            .filter(attached.gt(self.time))
            .order(attached)
            .first::<Attachment>(conn)
            .optional()?)
    }

    /// Return Attachment if self.part_id is attached somewhere at the event
    fn at(&self, conn: &AppConn) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(attached.le(self.time))
            .filter(detached.gt(self.time))
            .first::<Attachment>(conn)
            .optional()?)
    }

    /// Return Attachment if self.part_id is attached somewhere after the event
    fn after(&self, conn: &AppConn) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(attached.gt(self.time))
            .order(attached)
            .first::<Attachment>(conn)
            .optional()?)
    }

    /// Iff self.part_id already attached just before self.time return that attachment
    fn adjacent(&self, conn: &AppConn) -> TbResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(gear.eq(self.gear))
            .filter(hook.eq(self.hook))
            .filter(detached.eq(self.time))
            .first::<Attachment>(conn)
            .optional()?)
    }

    /// find all subparts of self which are attached to target at self.time
    fn assembly(&self, target: PartId, conn: &AppConn) -> TbResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;

        let types = self.part_id.what(conn)?.subtypes(conn);

        Ok(Attachment::belonging_to(&types)
            .for_update()
            .filter(gear.eq(target))
            .filter(attached.le(self.time))
            .filter(detached.gt(self.time))
            .order(hook)
            .load(conn)?)
    }
}
/// Timeline of attachments
///
/// * Every attachment of a part to a specified hook on a gear is an entry
/// * Start and end time are noted
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
    detached: DateTime<Utc>,
    // we do not accept theses values from the client!
    /// usage count
    pub count: i32,
    /// usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
}
/// Attachment with additional details
///
/// * the name is needed for attachments to parts which were sold
/// since the part will not be send to the client
/// * 'what' is an optimization
#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct AttachmentDetail {
    #[serde(flatten)]
    a: Attachment,
    name: String,
    what: PartTypeId,
}

impl AttachmentDetail {
    /// create a unique index for the attachment
    pub fn idx(&self) -> String {
        format!("{}{}", self.a.part_id, self.a.attached)
    }
}

impl Attachment {
    /// return the usage for the attachment
    fn usage(&self, factor: Factor, conn: &AppConn) -> Usage {
        Activity::find(self.gear, self.attached, self.detached, conn)
            .into_iter()
            .fold(Usage::none(self.attached), |acc, x| {
                acc.add_activity(&x, factor)
            })
    }

    fn shift (&self, at_time: DateTime<Utc>, target: PartId, hash: &mut SumHash, conn: &AppConn) -> TbResult<DateTime<Utc>> {
        debug!("-- moving {} to {}", self.part_id, target);
        let ev = Event {
            time: at_time,
            gear: target,
            part_id: self.part_id,
            hook: self.hook,
        };
        hash.merge(self.detach(at_time, conn)?);
        let (sum, det) = ev.attach_one(conn)?;
        hash.merge(sum);
        Ok(det)
    }
        
    /// change detached time for attachment
    ///
    /// * deletes the attachment for detached < attached
    /// * Does not check for collisions
    fn detach(mut self, detached: DateTime<Utc>, conn: &AppConn) -> TbResult<Summary> {
        trace!("detaching {} at {}", self.part_id, detached);

        let del = self.delete(conn)?;
        if self.attached >= detached {
            return Ok(del);
        }

        self.detached = detached;
        let cre = self.create(conn)?;
        Ok(del.merge(cre))
    }

    /// register and store a new attachment
    //
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    fn create(mut self, conn: &AppConn) -> TbResult<Summary> {
        trace!("create {:?}", self);
        let usage = self.usage(Factor::Add, conn);
        self.count = usage.count;
        self.time = usage.time;
        self.distance = usage.distance;
        self.climb = usage.climb;
        self.descend = usage.descend;
        let part = self.part_id.apply_usage(&usage, conn)?;

        let attachment = self
            .insert_into(attachments::table)
            .get_result::<Attachment>(conn)
            .context("insert into attachments")?
            .add_details(&part.name, part.what);

        Ok(Summary {
            parts: vec![part],
            attachments: vec![attachment],
            ..Default::default()
        })
    }

    /// deletes an attachment with its side-effects
    ///
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    fn delete(self, conn: &AppConn) -> TbResult<Summary> {
        trace!("delete {:?}", self);
        let ctx = format!("Could not delete attachment {:#?}", self);
        let mut att = diesel::delete(attachments::table.find(self.id())) // delete the attachment in the database
            .get_result::<Attachment>(conn)
            .context(ctx)?;

        let usage = att.usage(Factor::Sub, conn);
        let part = att.part_id.apply_usage(&usage, conn)?;
        att.count = 0;
        att.time = 0;
        att.distance = 0;
        att.climb = 0;
        att.descend = 0;

        // mark as deleted for client!
        att.detached = att.attached;
        return Ok(Summary {
            attachments: vec![att.add_details("".into(), 0.into())],
            parts: vec![part],
            activities: vec![],
        });
    }

    /// add redundant details for client simplicity
    fn add_details(self, name: &str, what: PartTypeId) -> AttachmentDetail {
        AttachmentDetail {
            name: name.to_string(),
            what,
            a: self,
        }
    }

    /// add redundant details from database for client simplicity
    fn read_details(self, conn: &AppConn) -> TbResult<AttachmentDetail> {
        use schema::parts::dsl::{name, parts, what};

        let (n, w) = parts
            .find(self.part_id)
            .select((name, what))
            .get_result::<(String, PartTypeId)>(conn)?;
        Ok(self.add_details(&n, w))
    }

    /// return all parts which are affected byActivity 'act'
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

    /// apply usage to all attachments affected by activity
    ///
    /// returns the list of Attechments - including the redundant details
    pub fn register(act: &Activity, usage: &Usage, conn: &AppConn) -> Vec<AttachmentDetail> {
        use schema::attachments::dsl::*;

        if let Some(act_gear) = act.gear {
            diesel::update(
                attachments
                    .filter(gear.eq(act_gear))
                    .filter(attached.lt(act.start))
                    .filter(detached.ge(act.start)),
            )
            .set((
                time.eq(time + usage.time),
                climb.eq(climb + usage.climb),
                descend.eq(descend + usage.descend),
                distance.eq(distance + usage.distance),
                count.eq(count + usage.count),
            ))
            .get_results::<Attachment>(conn)
            .expect("Database Error")
            .into_iter()
            .map(|a| a.read_details(conn).expect("couldn't enrich attachment"))
            .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    }

    /// return all attachments with details for the parts in 'partlist'
    pub fn for_parts(partlist: &Vec<Part>, conn: &AppConn) -> TbResult<Vec<AttachmentDetail>> {
        use schema::attachments::dsl::*;
        use schema::parts::dsl::{id, name, parts, what};
        let ids: Vec<_> = partlist.iter().map(|p| p.id).collect();
        let atts = attachments
            .filter(part_id.eq_any(ids.clone()))
            .or_filter(gear.eq_any(ids))
            .inner_join(parts.on(id.eq(part_id)))
            .select((schema::attachments::all_columns, name, what))
            .get_results::<AttachmentDetail>(conn)?;

        Ok(atts)
    }
}

/// route for attach API
#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().attach(user, &conn))
}

/// route for detach API
#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().detach(user, &conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
