/*
   tendabike - the bike maintenance tracker
   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! This module contains the implementation of the `Attachment` struct and its related functions.
//!
//! An attachment records that a part is attached to a gear at a certain time. Attachments can be hierarchical and are identified by part_id and attached time.
//!
//! This module also contains the implementation of the `Event` struct, which describes an attach or detach request.
//!

use super::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use schema::attachments;
use time::{macros::datetime, OffsetDateTime};

const MAX_TIME: OffsetDateTime = datetime!(9100-01-01 0:00 UTC);
/// Description of an Attach or Detach request
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Event {
    /// the part which should be change
    part_id: PartId,
    /// when it the change happens
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
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
    pub async fn detach(self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<Summary> {
        info!("detach {:?}", self);
        // check user
        self.part_id.checkuser(user, conn).await?;
        conn.transaction(|conn| {
            async move {
                let target = self
                    .at(conn)
                    .await?
                    .ok_or(Error::NotFound("part not attached".into()))?;

                ensure!(
                    self.hook == target.hook && self.gear == target.gear,
                    Error::BadRequest(format!("{:?} does not match attachment", self))
                );

                self.detach_assembly(target, conn).await
            }
            .scope_boxed()
        })
        .await
    }

    /// detach the whole assembly pointed at by 'self'
    ///
    /// 'target' is the corresponding Attachment. handed in as an optimization
    /// it must be the same as self.at(conn)
    ///
    /// When the 'self.partid' has child parts they are attached to that part
    ///
    async fn detach_assembly(self, target: Attachment, conn: &mut AppConn) -> AnyResult<Summary> {
        debug!("- detaching {}", target.part_id);
        let subs = self.assembly(target.gear, conn).await?;
        let mut hash = SumHash::new(target.detach(self.time, conn).await?);
        for sub in subs {
            sub.shift(self.time, target.part_id, &mut hash, conn)
                .await?;
        }
        Ok(hash.collect())
    }

    /// Create an attachment for 'self.part' and all it's childs
    /// It will detach any part - and childs therof - which art attached already
    ///
    /// When the detached part has child parts they are attached to that part
    ///
    pub async fn attach(self, user: &dyn Person, conn: &mut AppConn) -> AnyResult<Summary> {
        info!("attach {:?}", self);
        // check user
        let part = self.part_id.part(user, conn).await?;
        // and types
        let mytype = part.what.get(conn).await?;
        ensure!(
            mytype.hooks.contains(&self.hook),
            Error::BadRequest(format!(
                "Type {} cannot be attached to hook {}",
                mytype.name, self.hook
            ))
        );
        let gear = self.gear.part(user, conn).await?;
        ensure!(
            mytype.main == gear.what || mytype.hooks.contains(&gear.what),
            Error::BadRequest(format!(
                "Type {} cannot be attached to gear {}",
                mytype.name, gear.what
            ))
        );
        conn.transaction(|conn| {
            async move {
                let mut hash = SumHash::default();

                // detach self assembly
                if let Some(target) = self.at(conn).await? {
                    info!("detaching self assembly");
                    hash.merge(self.detach_assembly(target, conn).await?);
                }

                // detach target assembly
                if let Some(att) = self.occupant(conn).await? {
                    info!("detaching target assembly {}", att.part_id);
                    hash.merge(self.detach_assembly(att, conn).await?);
                }

                let subs = self.assembly(self.part_id, conn).await?;
                // reattach the assembly
                info!("attaching assembly");
                debug!("- attaching {}", self.part_id);
                let (sum, det) = self.attach_one(conn).await?;
                hash.merge(sum);
                for att in subs {
                    let sub_det = att.shift(self.time, self.gear, &mut hash, conn).await?;
                    if sub_det == det && det < att.detached {
                        trace!("reattaching {} to {} at {}", att.part_id, self.part_id, det);
                        let ev = Event {
                            part_id: att.part_id,
                            hook: att.hook,
                            gear: self.part_id,
                            time: det,
                        };
                        let (sum, _) = ev.attach_one(conn).await?;
                        hash.merge(sum);
                    }
                }
                Ok(hash.collect())
            }
            .scope_boxed()
        })
        .await
    }

    /// create Attachment for one part according to self
    ///
    /// * The part must not be attached somewhere at the event time
    /// * Also the hook must not be occupied at the event time
    /// * Detach time is adjusted according to later attachments
    ///
    /// If the part is attached already to the same hook, the attachments are merged
    async fn attach_one(self, conn: &mut AppConn) -> AnyResult<(Summary, OffsetDateTime)> {
        let mut hash = SumHash::default();
        // when does the current attachment end
        let mut end = MAX_TIME;
        // the time the current part will be detached
        // we need this to reattach subparts
        let mut det = MAX_TIME;

        let what = self.part_id.what(conn).await?;

        if let Some(next) = self.next(what, conn).await? {
            trace!("successor at {}", next.attached);
            // something else is already attached to the hook
            // the new attachment ends when the next starts
            end = next.attached;
            det = next.attached;
        }

        if let Some(next) = self.after(conn).await? {
            if end > next.attached {
                // is this attachment earlier than the previous one?
                if next.gear == self.gear && next.hook == self.hook {
                    trace!("still attached until {}", next.detached);
                    // the previous one is the real next so we keep 'det'!
                    // 'next' will be replaced by 'self' but 'end' is taken from 'next'
                    end = next.detached;
                    let sum = next.delete(conn).await?;
                    hash.merge(sum);
                } else {
                    trace!(
                        "changing gear/hook from {}/{} to {}/{}",
                        self.gear,
                        self.hook,
                        next.gear,
                        next.hook
                    );
                    // it is attached to a different hook later
                    // the new attachment ends when the next starts
                    end = next.attached;
                    det = next.attached
                }
            }
        }

        // try to merge previous attachment
        if let Some(prev) = self.adjacent(conn).await? {
            trace!("adjacent starting {}", prev.attached);
            hash.merge(prev.detach(end, conn).await?)
        } else {
            trace!("create {:?}\n", self);
            hash.merge(self.attachment(end).create(conn).await?);
        }

        Ok((hash.collect(), det))
    }

    /// create an attachment of self with the given detached time
    fn attachment(self, detached: OffsetDateTime) -> Attachment {
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
    async fn occupant(&self, conn: &mut AppConn) -> AnyResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        use schema::parts;
        let what = self.part_id.what(conn).await?;

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
            .await
            .optional()?)
    }

    /// Return Attachment if some other part is attached to same hook after the Event
    async fn next(&self, what: PartTypeId, conn: &mut AppConn) -> AnyResult<Option<Attachment>> {
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
            .await
            .optional()?)
    }

    /// Return Attachment if self.part_id is attached somewhere at the event
    async fn at(&self, conn: &mut AppConn) -> AnyResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(attached.le(self.time))
            .filter(detached.gt(self.time))
            .first::<Attachment>(conn)
            .await
            .optional()?)
    }

    /// Return Attachment if self.part_id is attached somewhere after the event
    async fn after(&self, conn: &mut AppConn) -> AnyResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(attached.gt(self.time))
            .order(attached)
            .first::<Attachment>(conn)
            .await
            .optional()?)
    }

    /// Iff self.part_id already attached just before self.time return that attachment
    async fn adjacent(&self, conn: &mut AppConn) -> AnyResult<Option<Attachment>> {
        use schema::attachments::dsl::*;
        Ok(attachments
            .for_update()
            .filter(part_id.eq(self.part_id))
            .filter(gear.eq(self.gear))
            .filter(hook.eq(self.hook))
            .filter(detached.eq(self.time))
            .first::<Attachment>(conn)
            .await
            .optional()?)
    }

    /// find all subparts of self which are attached to target at self.time
    async fn assembly(&self, target: PartId, conn: &mut AppConn) -> AnyResult<Vec<Attachment>> {
        use schema::attachments::dsl::*;

        let types = self.part_id.what(conn).await?.subtypes(conn).await;

        Ok(Attachment::belonging_to(&types)
            .for_update()
            .filter(gear.eq(target))
            .filter(attached.le(self.time))
            .filter(detached.gt(self.time))
            .order(hook)
            .load(conn)
            .await?)
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
#[diesel(primary_key(part_id, attached))]
#[diesel(treat_none_as_null = true)]
#[diesel(belongs_to(PartType, foreign_key = hook))]
pub struct Attachment {
    /// the sub-part, which is attached to the hook
    part_id: PartId,
    /// when it was attached
    #[serde(with = "time::serde::rfc3339")]
    attached: OffsetDateTime,
    /// The gear the part is attached to
    gear: PartId,
    /// the hook on that gear
    hook: PartTypeId,
    /// when it was removed again, "none" means "still attached"
    #[serde(with = "time::serde::rfc3339")]
    detached: OffsetDateTime,
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
    async fn usage(&self, factor: Factor, conn: &mut AppConn) -> AnyResult<Usage> {
        Ok(Activity::find(self.gear, self.attached, self.detached, conn).await?
            .into_iter()
            .fold(Usage::none(), |acc, x| acc.add_activity(&x, factor)))
    }

    async fn shift(
        &self,
        at_time: OffsetDateTime,
        target: PartId,
        hash: &mut SumHash,
        conn: &mut AppConn,
    ) -> AnyResult<OffsetDateTime> {
        debug!("-- moving {} to {}", self.part_id, target);
        let ev = Event {
            time: at_time,
            gear: target,
            part_id: self.part_id,
            hook: self.hook,
        };
        hash.merge(self.detach(at_time, conn).await?);
        let (sum, det) = ev.attach_one(conn).await?;
        hash.merge(sum);
        Ok(det)
    }

    /// change detached time for attachment
    ///
    /// * deletes the attachment for detached < attached
    /// * Does not check for collisions
    async fn detach(mut self, detached: OffsetDateTime, conn: &mut AppConn) -> AnyResult<Summary> {
        trace!("detaching {} at {}", self.part_id, detached);

        let del = self.delete(conn).await?;
        if self.attached >= detached {
            return Ok(del);
        }

        self.detached = detached;
        let cre = self.create(conn).await?;
        Ok(del.merge(cre))
    }

    /// register and store a new attachment
    //
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    async fn create(mut self, conn: &mut AppConn) -> AnyResult<Summary> {
        trace!("create {:?}", self);
        let usage = self.usage(Factor::Add, conn).await?;
        self.count = usage.count;
        self.time = usage.time;
        self.distance = usage.distance;
        self.climb = usage.climb;
        self.descend = usage.descend;
        let part = self
            .part_id
            .apply_usage(&usage, self.attached, conn)
            .await?;

        let attachment = self
            .insert_into(attachments::table)
            .get_result::<Attachment>(conn)
            .await
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
    async fn delete(self, conn: &mut AppConn) -> AnyResult<Summary> {
        trace!("delete {:?}", self);
        let ctx = format!("Could not delete attachment {:#?}", self);
        let mut att = diesel::delete(attachments::table.find(self.id())) // delete the attachment in the database
            .get_result::<Attachment>(conn)
            .await
            .context(ctx)?;

        let usage = att.usage(Factor::Sub, conn).await?;
        let part = att.part_id.apply_usage(&usage, att.attached, conn).await?;
        att.count = 0;
        att.time = 0;
        att.distance = 0;
        att.climb = 0;
        att.descend = 0;

        // mark as deleted for client!
        att.detached = att.attached;
        Ok(Summary {
            attachments: vec![att.add_details("", 0.into())],
            parts: vec![part],
            activities: vec![],
        })
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
    async fn read_details(self, conn: &mut AppConn) -> AnyResult<AttachmentDetail> {
        use schema::parts::dsl::{name, parts, what};

        let (n, w) = parts
            .find(self.part_id)
            .select((name, what))
            .get_result::<(String, PartTypeId)>(conn)
            .await?;
        Ok(self.add_details(&n, w))
    }

    /// return all parts which are affected byActivity 'act'
    pub async fn parts_per_activity(act: &Activity, conn: &mut AppConn) -> Vec<PartId> {
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
                    .await
                    .expect("Error reading attachments"),
            );
        }
        res
    }

    /// apply usage to all attachments affected by activity
    ///
    /// returns the list of Attachments - including the redundant details
    pub async fn register(
        act: &Activity,
        usage: &Usage,
        conn: &mut AppConn,
    ) -> Vec<AttachmentDetail> {
        use schema::attachments::dsl::*;

        let mut res = Vec::new();
        if let Some(act_gear) = act.gear {
            let atts = diesel::update(
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
            .await
            .expect("Database Error");
            for att in atts.iter() {
                res.push(
                    att.read_details(conn)
                        .await
                        .expect("couldn't enrich attachment"),
                );
            }
        }
        res
    }

    /// return all attachments with details for the parts in 'partlist'
    pub async fn for_parts(
        partlist: &[Part],
        conn: &mut AppConn,
    ) -> AnyResult<Vec<AttachmentDetail>> {
        use schema::attachments::dsl::*;
        use schema::parts::dsl::{id, name, parts, what};
        let ids: Vec<_> = partlist.iter().map(|p| p.id).collect();
        let atts = attachments
            .filter(part_id.eq_any(ids.clone()))
            .or_filter(gear.eq_any(ids))
            .inner_join(parts.on(id.eq(part_id)))
            .select((schema::attachments::all_columns, name, what))
            .get_results::<AttachmentDetail>(conn)
            .await?;

        Ok(atts)
    }
}
