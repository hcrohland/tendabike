use anyhow::ensure;
use async_session::log::{info, debug, trace};
use diesel::prelude::*; 
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use s_diesel::{AppConn, schema};
use serde_derive::Deserialize;
use time::OffsetDateTime;

use crate::{PartId, PartTypeId, Person, AnyResult, Summary, Error, Attachment, SumHash};

const MAX_TIME: OffsetDateTime = time::macros::datetime!(9100-01-01 0:00 UTC);

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
    /// Create a new Event
    /// 
    pub fn new(part_id: PartId, time: OffsetDateTime, gear: PartId, hook: PartTypeId) -> Self {
        Self {
            part_id,
            time,
            gear,
            hook,
        }
    }

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
    async fn detach_assembly(
        self,
        target: Attachment,
        conn: &mut AppConn,
    ) -> AnyResult<Summary> {
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
    pub(super) async fn attach_one(self, conn: &mut AppConn) -> AnyResult<(Summary, OffsetDateTime)> {
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
    async fn next(
        &self,
        what: PartTypeId,
        conn: &mut AppConn,
    ) -> AnyResult<Option<Attachment>> {
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
