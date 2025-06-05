use async_session::log::{debug, trace};
use scoped_futures::ScopedFutureExt;
use serde_derive::Deserialize;
use time::OffsetDateTime;

use crate::*;

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
    /// if true, the the whole assembly will be detached
    all: bool,
}

impl Event {
    /// Create a new Event
    ///
    pub fn new(
        part_id: PartId,
        time: OffsetDateTime,
        gear: PartId,
        hook: PartTypeId,
        all: bool,
    ) -> Self {
        Self {
            part_id,
            time,
            gear,
            hook,
            all,
        }
    }

    /// End an attachment for 'self.part' and all it's childs
    ///
    /// Check's authorization and that the part is attached
    ///
    pub async fn detach(self, user: &dyn Person, store: &mut impl Store) -> TbResult<Summary> {
        debug!("detach {:?}", self);
        // check user
        self.part_id.checkuser(user, store).await?;
        store
            .transaction(|store| {
                async move {
                    let attachment = store
                        .attachment_get_by_part_and_time(self.part_id, self.time)
                        .await?
                        .ok_or(Error::NotFound("part not attached".into()))?;

                    if !(self.hook == attachment.hook && self.gear == attachment.gear) {
                        return Err(Error::BadRequest(format!(
                            "{:?} does not match attachment",
                            self
                        )));
                    }

                    attachment.detach_assembly(self.time, self.all, store).await
                }
                .scope_boxed()
            })
            .await
    }

    /// Create an attachment for 'self.part' and all it's childs
    /// It will detach any part - and childs therof - which are attached already
    ///
    /// When the detached part has child parts they are attached to that part
    ///
    pub async fn attach(
        self: Event,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Summary> {
        debug!("attach {:?}", self);

        let Event {
            part_id,
            time,
            gear,
            hook,
            all,
        } = self;
        // check user
        let part = part_id.part(user, store).await?;
        // and types
        let parttype = part.what.get()?;
        if !parttype.hooks.contains(&hook) {
            return Err(Error::BadRequest(format!(
                "Type {} cannot be attached to hook {}",
                parttype.name, hook
            )));
        };
        let geartype = gear.part(user, store).await?.what;
        if !(parttype.main == geartype || parttype.hooks.contains(&geartype)) {
            return Err(Error::BadRequest(format!(
                "Type {} cannot be attached to gear type {}",
                parttype.name,
                geartype
                    .get()
                    .map(|t| t.name)
                    .unwrap_or_else(|_| format!("unknown type {}", geartype))
            )));
        };
        store
            .transaction(|store| {
                async move {
                    let mut hash = SumHash::default();

                    // detach part if it is attached already
                    if let Some(attachment) =
                        store.attachment_get_by_part_and_time(part_id, time).await?
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
                    debug!("- attaching assembly {} to {}", part_id, gear);
                    let end =
                        super::attach_one(part_id, time, gear, hook, &mut hash, store).await?;
                    if all {
                        let subparts = super::subparts(part_id, part_id, time, store).await?;
                        for attachment in dbg!(subparts) {
                            let detached = attachment.shift(time, gear, &mut hash, store).await?;
                            if detached == end && end < attachment.detached {
                                trace!(
                                    "reattaching {} to {} at {}",
                                    attachment.part_id, part_id, end
                                );

                                super::attach_one(
                                    attachment.part_id,
                                    end,
                                    part_id,
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
}
