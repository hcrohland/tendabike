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

use serde_derive::{Deserialize, Serialize};

use crate::traits::Store;

use crate::*;
use time::OffsetDateTime;

pub use event::Event;
mod event;

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
/// since the part will not be send to the client
/// * 'what' is an optimization
#[derive(Queryable, Serialize, Deserialize, Debug)]
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

    async fn shift(
        &self,
        at_time: OffsetDateTime,
        target: PartId,
        hash: &mut SumHash,
        store: &mut impl Store,
    ) -> TbResult<OffsetDateTime> {
        debug!("-- moving {} to {}", self.part_id, target);
        let ev = Event::new(self.part_id, at_time, target, self.hook);
        hash.merge(self.detach(at_time, store).await?);
        let (sum, det) = ev.attach_one(store).await?;
        hash.merge(sum);
        Ok(det)
    }

    /// change detached time for attachment
    ///
    /// * deletes the attachment for detached < attached
    /// * Does not check for collisions
    async fn detach(
        mut self,
        detached: OffsetDateTime,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        trace!("detaching {} at {}", self.part_id, detached);

        let del = self.delete(store).await?;
        if self.attached >= detached {
            return Ok(del);
        }

        self.detached = detached;
        let cre = self.create(store).await?;
        Ok(del.merge(cre))
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
        let part_usage = part.usage(store).await? + &usage;

        // Store both usages.
        Usage::update_vec(&vec![&usage, &part_usage], store).await?;

        // store the attachment in the database
        let attachment = store
            .attachment_create(self)
            .await?
            .add_details(&part.name, part.what);

        // return all affected objects
        Ok(Summary {
            parts: vec![part],
            attachments: vec![attachment],
            usages: vec![usage, part_usage],
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
        let att = store.attachment_delete(self).await?;
        let usage = -att.usage.delete(store).await?;

        // adjust part usage and store it
        let part_usage = att.part_id.read(store).await?.usage(store).await?;
        let usages = vec![part_usage + &usage];
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
        let attachments = store.attachment_get_by_gear_and_time(gear, start).await?;
        let mut usages = Vec::new();
        for att in attachments.iter() {
            usages.push(att.usage(store).await? + &usage);
        }

        // get all parts from attachments, add usage and modify last_used
        let partlist = attachments.iter().map(|a| a.part_id);
        let mut parts = Vec::new();
        for part in partlist {
            let part = part.update_last_use(start, store).await?;
            usages.push(part.usage(store).await? + &usage);
            parts.push(part);
        }

        // store all updated usages
        Usage::update_vec(&usages, store).await?;
        Ok(Summary {
            usages,
            parts,
            ..Default::default()
        })
    }
}
