//! The list of all historical attachments. This is the central piece of TendaBike.
//!
//! Attachments can be hierarchical
//! They are identified by part_id and attached time

use chrono::MAX_DATETIME;
use diesel::{self, QueryDsl, RunQueryDsl};
use rocket_contrib::json::Json;

use crate::*;
use schema::attachments;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Event {
    /// the sub-part, which is attached to the hook
    part_id: PartId,
    /// when it was attached
    time: DateTime<Utc>,
    /// The gear the part will be attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
}

impl Event {
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

    fn detach_assembly(self, target: Attachment, conn: &AppConn) -> TbResult<Summary> {
        debug!("- detaching {}", target.part_id);
        let mut hash = SumHash::new(target.detach(self.time, conn)?);
        let subs = self.assembly(target.gear, conn)?;
        for att in subs {
            let ev = Event {
                part_id: att.part_id,
                time: self.time,
                gear: target.part_id,
                hook: att.hook,
            };
            debug!(
                "-- detaching {} from {} to {}",
                att.part_id, target.gear, ev.gear
            );
            hash.merge(att.detach(self.time, conn)?);
            hash.merge(ev.attach_one(conn)?);
        }
        Ok(hash.collect())
    }

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
            hash.merge(self.attach_one(conn)?);
            for att in subs {
                let ev = Event {
                    part_id: att.part_id,
                    gear: self.gear,
                    time: self.time,
                    hook: att.hook,
                };
                debug!("-- moving {} from {} to {}", att.part_id, att.gear, ev.gear);
                hash.merge(att.detach(self.time, conn)?);
                hash.merge(ev.attach_one(conn)?);
            }
            Ok(hash.collect())
        })
    }

    fn attach_one(self, conn: &AppConn) -> TbResult<Summary> {
        let mut hash = SumHash::default();
        let mut end = MAX_DATETIME;

        let what = self.part_id.what(conn)?;

        if let Some(next) = self.next(what, conn)? {
            trace!("successor at {}", next.detached);
            // something else is already attached to the hook
            // the new attachment ends when the next starts
            end = next.attached;
        }

        if let Some(next) = self.after(conn)? {
            if next.gear == self.gear && next.hook == self.hook {
                trace!("still attached until {}", next.detached);
                // next will be superseded by self
                // but the end is taken from next
                end = next.detached;
                let sum = next.delete(conn)?;
                hash.merge(sum);
            } else {
                trace!("changing hook {}", next.hook);
                // it is attached to a different hook later
                // the new attachment ends when the next starts
                end = next.attached;
            }
        }

        // try to merge previous attachment
        if let Some(prev) = self.adjacent(conn)? {
            trace!("adjacent starting {}", prev.attached);
            hash.merge(prev.detach(end, conn)?)
        } else {
            debug!("create {:?}\n", self);
            hash.merge(self.attachment(end).create(conn)?);
        }

        Ok(hash.collect())
    }

    /// create an attachment our of self with the given detached time
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

    /// find other parts attached to same hook at Event
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

    /// find other parts attached to same hook after the Event
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

    /// is self.part_id attached somewhere at the event
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

    /// is self.part_id attached somewhere after the event
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

    /// is self.part_id already attached just before self.time
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

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct AttachmentDetail {
    #[serde(flatten)]
    a: Attachment,
    name: String,
    what: PartTypeId,
}

impl AttachmentDetail {
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

    /// change detached time for attachment
    ///
    /// deletes the attachment for detached < attached
    /// Does not check for collisions
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
    fn create(mut self, conn: &AppConn) -> TbResult<Summary> {
        trace!("create {:?}", self);
        let usage = self.usage(Factor::Add, conn);
        self.count = usage.count;
        self.time = usage.time;
        self.distance = usage.distance;
        self.climb = usage.climb;
        self.descend = usage.descend;
        let part = self.part_id.apply_usage(&usage, conn)?;

        let a = self
            .insert_into(attachments::table)
            .get_result(conn)
            .context("insert into attachments")?;
        let attachment = AttachmentDetail {
            a,
            name: part.name.clone(),
            what: part.what,
        };
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
    fn add_details(self, name: String, what: PartTypeId) -> AttachmentDetail {
        AttachmentDetail {
            name,
            what,
            a: self,
        }
    }

    /// read and add redundant details for client simplicity
    fn detail(self, conn: &AppConn) -> TbResult<AttachmentDetail> {
        use schema::parts::dsl::{name, parts, what};

        let (n, w) = parts
            .find(self.part_id)
            .select((name, what))
            .get_result::<(String, PartTypeId)>(conn)?;
        Ok(self.add_details(n, w))
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
            .map(|a| a.detail(conn).expect("couldn't enrich attachment"))
            .collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    }

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

#[post("/attach", data = "<event>")]
fn attach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().attach(user, &conn))
}

#[post("/detach", data = "<event>")]
fn detach_rt(event: Json<Event>, user: &User, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(event.into_inner().detach(user, &conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![attach_rt, detach_rt]
}
