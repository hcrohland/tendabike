use anyhow::ensure;
use async_session::log::{debug, trace};
use scoped_futures::ScopedFutureExt;
use serde_derive::Deserialize;
use time::OffsetDateTime;

use crate::{
    traits::Store, AnyResult, Attachment, Error, PartId, PartTypeId, Person, SumHash, Summary,
};

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
    pub async fn detach(self, user: &dyn Person, conn: &mut impl Store) -> AnyResult<Summary> {
        debug!("detach {:?}", self);
        // check user
        self.part_id.checkuser(user, conn).await?;
        conn.storetransaction(|conn| {
            async move {
                let target = conn
                    .attachment_get_by_part_and_time(self.part_id, self.time)
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
        conn: &mut impl Store,
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
    pub async fn attach(self, user: &dyn Person, conn: &mut impl Store) -> AnyResult<Summary> {
        debug!("attach {:?}", self);
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
        conn.storetransaction(|conn| {
            async move {
                let mut hash = SumHash::default();

                // detach self assembly
                if let Some(target) = conn
                    .attachment_get_by_part_and_time(self.part_id, self.time)
                    .await?
                {
                    debug!("detaching self assembly");
                    hash.merge(self.detach_assembly(target, conn).await?);
                }

                // detach target assembly
                let what = self.part_id.what(conn).await?;
                let attachment = conn
                    .attachment_find_part_of_type_at_hook_and_time(
                        what, self.gear, self.hook, self.time,
                    )
                    .await?;
                if let Some(att) = attachment {
                    debug!("detaching target assembly {}", att.part_id);
                    hash.merge(self.detach_assembly(att, conn).await?);
                }

                let subs = self.assembly(self.part_id, conn).await?;
                // reattach the assembly
                debug!("- attaching assembly {} to {}", self.part_id, self.gear);
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
    pub(super) async fn attach_one(
        self,
        conn: &mut impl Store,
    ) -> AnyResult<(Summary, OffsetDateTime)> {
        let mut hash = SumHash::default();
        // when does the current attachment end
        let mut end = MAX_TIME;
        // the time the current part will be detached
        // we need this to reattach subparts
        let mut det = MAX_TIME;

        let what = self.part_id.what(conn).await?;

        if let Some(next) = conn
            .attachment_find_successor(self.part_id, self.gear, self.hook, self.time, what)
            .await?
        {
            trace!("successor at {}", next.attached);
            // something else is already attached to the hook
            // the new attachment ends when the next starts
            end = next.attached;
            det = next.attached;
        }

        if let Some(next) = conn
            .attachment_find_later_attachment_for_part(self.part_id, self.time)
            .await?
        {
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
        if let Some(prev) = conn
            .attachment_find_part_attached_already(self.part_id, self.gear, self.hook, self.time)
            .await?
        {
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

    /// find all subparts of self which are attached to target at self.time
    async fn assembly(&self, target: PartId, conn: &mut impl Store) -> AnyResult<Vec<Attachment>> {
        let types = self.part_id.what(conn).await?.subtypes(conn).await;
        conn.assembly_get_by_types_time_and_gear(types, target, self.time)
            .await
    }
}
