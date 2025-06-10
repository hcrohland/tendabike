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

use scoped_futures::ScopedFutureExt;
use serde_derive::{Deserialize, Serialize};

use crate::traits::{AttachmentStore, Store};

use crate::*;
use time::OffsetDateTime;

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
    Insertable,
    AsChangeset,
)]
#[diesel(primary_key(part_id, attached))]
#[diesel(treat_none_as_null = true)]
#[diesel(table_name = schema::attachments)]
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
    usage: UsageId,
}
/// Attachment with additional details
///
/// * the name is needed for attachments to parts which were sold
///   since the part will not be send to the client
/// * 'what' is an optimization
#[derive(Queryable, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AttachmentDetail {
    #[serde(flatten)]
    pub a: Attachment,
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
    /// Create a new attachment
    ///
    fn new(
        part_id: PartId,
        attached: OffsetDateTime,
        gear: PartId,
        hook: PartTypeId,
        detached: OffsetDateTime,
    ) -> Self {
        Self {
            part_id,
            attached,
            gear,
            hook,
            detached,
            usage: UsageId::new(),
        }
    }
    /// return the calculated usage for the attachment
    async fn calculate_usage(&self, store: &mut impl ActivityStore) -> TbResult<Usage> {
        Ok(
            Activity::find(self.gear, self.attached, self.detached, store)
                .await?
                .into_iter()
                .fold(Usage::new(self.usage), |usage, act| usage + &act.usage()),
        )
    }

    pub(crate) async fn usage(&self, store: &mut impl UsageStore) -> TbResult<Usage> {
        self.usage.read(store).await
    }

    /// Move a single part to a new gear 'target' at a certain time
    ///
    /// updates hash with the changes
    /// returns the time the new attachment ends
    async fn shift(
        &self,
        time: OffsetDateTime,
        gear: PartId,
        hash: &mut SumHash,
        store: &mut impl Store,
    ) -> TbResult<OffsetDateTime> {
        debug!("-- moving {} to {}", self.part_id, gear);
        *hash += self.detach(time, store).await?;
        attach_one(self.part_id, time, gear, self.hook, hash, store).await
    }

    /// change detached time for attachment
    ///
    /// * deletes the attachment for detached < attached
    /// * Does not check for collisions
    async fn detach(mut self, time: OffsetDateTime, store: &mut impl Store) -> TbResult<Summary> {
        trace!("detaching {} at {}", self.part_id, time);

        // delete the old attachment
        let res = self.delete(store).await?;
        if self.attached >= time {
            // if it was detached at or before the attach time, we do not need to create a new attachment
            return Ok(res);
        }

        // create a new attachment with the new detached time
        self.detached = time;
        Ok(res + self.create(store).await?)
    }

    /// register and store a new attachment
    //
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    async fn create(mut self, store: &mut impl Store) -> TbResult<Summary> {
        trace!("create {:?}", self);

        // create the Usage for the attachement
        self.usage = UsageId::new();
        let usage = self.calculate_usage(store).await?;

        // add that usage to the part
        let part = self.part_id.update_last_use(self.attached, store).await?;
        let mut usages = vec![part.usage().read(store).await? + &usage, usage];
        // store the attachment in the database
        let attachment = store
            .attachment_create(self)
            .await?
            .add_details(&part.name, part.what);

        // recalculate the service usages and append to usages
        usages.append(&mut Service::recalculate(part.id, self.attached, store).await?);

        // Store all usages.
        Usage::update_vec(&usages, store).await?;

        // return all affected objects
        Ok(Summary {
            parts: vec![part],
            attachments: vec![attachment],
            usages,
            ..Default::default()
        })
    }

    /// deletes an attachment with its side-effects
    ///
    /// - recalculates the usage counters in the attached assembly
    /// - returns all affected parts
    async fn delete(self, store: &mut impl Store) -> TbResult<Summary> {
        trace!("delete {:?}", self);

        // delete the attachment on the db
        let att = AttachmentStore::delete(store, self).await?;
        let usage = -att.usage.delete(store).await?;

        // recalc service usages
        let mut usages = Service::recalculate(att.part_id, att.attached, store).await?;

        // adjust part usage
        usages.push(att.part_id.read(store).await?.usage().read(store).await? + &usage);

        // store all usages
        Usage::update_vec(&usages, store).await?;

        // mark attachment as deleted for client!
        let mut att = att;
        att.detached = att.attached;
        att.usage = UsageId::new();
        Ok(Summary {
            attachments: vec![att.add_details("", 0.into())],
            usages,
            ..Default::default()
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
    async fn read_details(self, store: &mut impl PartStore) -> TbResult<AttachmentDetail> {
        let part = self.part_id.read(store).await?;
        Ok(self.add_details(&part.name, part.what))
    }

    pub(crate) async fn activities_by_part(
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
        store: &mut (impl AttachmentStore + ActivityStore),
    ) -> TbResult<Vec<Activity>> {
        use std::cmp::{max, min};
        let attachments = store.attachments_all_by_part(part).await?;
        let mut activities = Vec::new();
        for att in attachments {
            let begin = max(att.attached, begin);
            let end = min(att.detached, end);
            activities.append(&mut Activity::find(att.gear, begin, end, store).await?);
        }
        Ok(activities)
    }

    /// return all attachments with details for the parts in 'partlist'
    pub(crate) async fn for_part_with_usage(
        part: PartId,
        store: &mut impl Store,
    ) -> TbResult<(Vec<AttachmentDetail>, Vec<Usage>)> {
        let atts = store.attachments_all_by_part(part).await?;

        let mut attachments = Vec::new();
        let mut usages = Vec::new();
        for att in atts {
            attachments.push(att.read_details(store).await?);
            usages.push(att.usage(store).await?);
        }
        Ok((attachments, usages))
    }

    pub(crate) async fn register_activity(
        gear: Option<PartId>,
        start: OffsetDateTime,
        usage: Usage,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        let gear = match gear {
            None => return Ok(Summary::default()),
            Some(x) => x,
        };

        // get all attachment usages and add usage to it
        let mut usages = Vec::new();
        let mut parts = Vec::new();

        let attachments = store.attachment_get_by_gear_and_time(gear, start).await?;
        for att in attachments.iter() {
            usages.push(att.usage);
        }

        // get all parts from attachments, add usage and modify last_used
        let partlist = attachments.iter().map(|a| a.part_id);
        // we need to add gear since it is not attached
        for part in partlist.chain([gear]) {
            let part = part.update_last_use(start, store).await?;
            usages.push(part.usage());
            usages.append(&mut Service::get_usageids(part.id, start, store).await?);
            parts.push(part);
        }

        let usages = Usage::get_vec(&usages, store).await? + &usage;
        // store all updated usages
        Usage::update_vec(&usages, store).await?;
        Ok(Summary {
            usages,
            parts,
            ..Default::default()
        })
    }

    async fn detach_assembly(
        self,
        time: OffsetDateTime,
        all: bool,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        debug!("-- detaching {} at {}", self.part_id, time);

        let mut hash = SumHash::default();
        if all {
            shift_subparts(self.gear, self.part_id, time, &mut hash, store).await?;
        }
        // detach the part
        hash += self.detach(time, store).await?;
        Ok(hash.into())
    }
}

/// moves all subparts of 'from' to 'to' at 'time'
///
/// This is used when the part is detached with all subparts
///
///  # Updates the hash of the changes
async fn shift_subparts(
    from: PartId,
    to: PartId,
    time: OffsetDateTime,
    hash: &mut SumHash,
    store: &mut impl Store,
) -> TbResult<()> {
    let sub_attachments = subparts(to, from, time, store).await?;
    for attachment in sub_attachments {
        attachment.shift(time, to, hash, store).await?;
    }
    Ok(())
}

/// find all subparts which are attached to target at self.time
async fn subparts(
    part: PartId,
    gear: PartId,
    time: OffsetDateTime,
    store: &mut impl Store,
) -> TbResult<Vec<Attachment>> {
    let types = part.what(store).await?.subtypes();
    store
        .assembly_get_by_types_time_and_gear(types, gear, time)
        .await
}

/// create Attachment for one part according to self
///
/// * The part must not be attached somewhere at the event time
/// * Also the hook must not be occupied at the event time
/// * Detach time is adjusted according to later attachments
///
/// If the part is attached already to the same hook, the attachments are merged
///
/// returns all affected entities and the time the attachment ends or an error
async fn attach_one(
    part_id: PartId,
    time: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
    hash: &mut SumHash,
    store: &mut impl Store,
) -> TbResult<OffsetDateTime> {
    // when does the current attachment end
    let mut end = MAX_TIME;
    // the time the current part will be detached
    // we need this to reattach subparts
    let mut det = MAX_TIME;

    let what = part_id.what(store).await?;

    if let Some(next) = store
        .attachment_find_successor(part_id, gear, hook, time, what)
        .await?
    {
        trace!("successor at {}", next.attached);
        // something else is already attached to the hook
        // the new attachment ends when the next starts
        end = next.attached;
        det = next.attached;
    }

    if let Some(next) = store
        .attachment_find_later_attachment_for_part(part_id, time)
        .await?
    {
        if end > next.attached {
            // is this attachment earlier than the previous one?
            if next.gear == gear && next.hook == hook {
                trace!("still attached until {}", next.detached);
                // the previous one is the real next so we keep 'det'!
                // 'next' will be replaced by 'self' but 'end' is taken from 'next'
                end = next.detached;
                *hash += next.delete(store).await?;
            } else {
                trace!(
                    "changing gear/hook from {}/{} to {}/{}",
                    gear, hook, next.gear, next.hook
                );
                // it is attached to a different hook later
                // the new attachment ends when the next starts
                end = next.attached;
                det = next.attached
            }
        }
    }

    // try to merge previous attachment
    match store
        .attachment_find_part_attached_already(part_id, gear, hook, time)
        .await?
    {
        Some(prev) => {
            trace!("adjacent starting {}", prev.attached);
            *hash += prev.detach(end, store).await?
        }
        _ => {
            // trace!("create {:?}\n", self);
            *hash += Attachment::new(part_id, time, gear, hook, end)
                .create(store)
                .await?;
        }
    }

    Ok(det)
}

pub async fn attach_assembly(
    user: &dyn Person,
    part: PartId,
    time: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
    all: bool,
    store: &mut impl Store,
) -> Result<Summary, Error> {
    let time = round_time(time);
    // check user
    let part = part.part(user, store).await?;
    let parttype = part.what.get()?;

    let geartypeid = gear.part(user, store).await?.what;

    if !parttype.hooks.contains(&hook) {
        return Err(Error::BadRequest(format!(
            "Type {} cannot be attached to hook {}",
            parttype.name, hook
        )));
    };
    if !(parttype.main == geartypeid || parttype.hooks.contains(&geartypeid)) {
        return Err(Error::BadRequest(format!(
            "Type {} cannot be attached to gear type {}",
            parttype.name,
            geartypeid
                .get()
                .map(|t| t.name)
                .unwrap_or_else(|_| format!("unknown type {}", geartypeid))
        )));
    };
    store
        .transaction(|store| {
            async move {
                let mut hash = SumHash::default();

                // detach part if it is attached already
                if let Some(attachment) =
                    store.attachment_get_by_part_and_time(part.id, time).await?
                {
                    debug!("detaching self assembly");
                    hash += attachment.detach_assembly(time, all, store).await?;
                }

                // if there is a part attached to the gear at the hook, detach it
                let attachment = store
                    .attachment_find_part_of_type_at_hook_and_time(part.what, gear, hook, time)
                    .await?;
                if let Some(attachment) = attachment {
                    debug!("detaching predecessor assembly {}", attachment.part_id);
                    hash += attachment.detach_assembly(time, all, store).await?;
                }

                // reattach the assembly
                debug!("- attaching assembly {} to {}", part.id, gear);
                let end = attach_one(part.id, time, gear, hook, &mut hash, store).await?;
                if all {
                    let subparts = subparts(part.id, part.id, time, store).await?;
                    for attachment in subparts {
                        let detached = attachment.shift(time, gear, &mut hash, store).await?;
                        if detached == end && end < attachment.detached {
                            trace!(
                                "reattaching {} to {} at {}",
                                attachment.part_id, part.id, end
                            );

                            attach_one(
                                attachment.part_id,
                                end,
                                part.id,
                                attachment.hook,
                                &mut hash,
                                store,
                            )
                            .await?;
                        }
                    }
                }
                Ok(hash.into())
            }
            .scope_boxed()
        })
        .await
}

pub async fn detach_assembly(
    user: &dyn Person,
    part_id: PartId,
    time: OffsetDateTime,
    all: bool,
    store: &mut impl Store,
) -> Result<Summary, Error> {
    let time = round_time(time);
    part_id.checkuser(user, store).await?;

    store
        .transaction(|store| {
            async move {
                let attachment = store
                    .attachment_get_by_part_and_time(part_id, time)
                    .await?
                    .ok_or(Error::NotFound("part not attached".into()))?;
                attachment.detach_assembly(time, all, store).await
            }
            .scope_boxed()
        })
        .await
}

pub async fn dispose_assembly(
    user: &dyn Person,
    part_id: PartId,
    time: OffsetDateTime,
    all: bool,
    store: &mut impl Store,
) -> Result<Summary, Error> {
    let time = round_time(time);

    part_id.checkuser(user, store).await?;

    let attachments = store.attachments_all_by_part(part_id).await?;

    if let Some(attachment) = attachments
        .iter()
        .find(|a| a.detached < MAX_TIME && a.detached > time)
    {
        return Err(Error::Conflict(format!(
            "Cannot dispose. {part_id} attached to {} after {time}",
            attachment.gear
        )));
    }

    let mut res = SumHash::default();
    res += part_id.dispose(time, store).await?;
    res += dispose_subparts(part_id, time, all, store).await?;

    Ok(res.into())
}

async fn dispose_subparts(
    part: PartId,
    time: OffsetDateTime,
    all: bool,
    store: &mut impl Store,
) -> TbResult<Summary> {
    let sub_attachments = subparts(part, part, time, store).await?;
    let mut res = SumHash::default();
    for attachment in sub_attachments {
        let attachments = store.attachments_all_by_part(attachment.part_id).await?;
        if !all || attachments.iter().any(|a| a.attached > time) {
            debug!("-- detaching {}", attachment.part_id);
            res += attachment.detach(time, store).await?
        } else {
            res += attachment.part_id.dispose(time, store).await?
        }
    }
    Ok(res.into())
}

pub async fn recover_assembly(
    user: &dyn Person,
    part: PartId,
    all: bool,
    store: &mut impl Store,
) -> Result<Summary, Error> {
    let mut res = SumHash::default();
    if let Some(time) = part.part(user, store).await?.disposed_at {
        res += part.restore(store).await?;
        if all {
            for attachment in subparts(part, part, time, store).await? {
                res += attachment.part_id.restore(store).await?;
            }
        }
        Ok(res.into())
    } else {
        Err(Error::BadRequest(format!("Part {part} is not disposed")))
    }
}
