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

//! The `ApiWrite` dispatch (ADR-0005, executable spec #446 §3).
//!
//! `ApiWrite` is the one in-memory message an API write becomes on its way
//! through the per-user executor: a flat enum with one variant per mutating
//! route, entity-prefixed, each variant carrying exactly the operation's
//! parsed arguments — the same values the current axum handlers extract.
//! Reads are not `ApiWrite` variants; they bypass the loop.
//!
//! No variant carries a `user_id`: the channel is per-user, the loop runs in
//! that user's context, and the `Session` the executor supplies carries the
//! identity.
//!
//! [`exec`] applies one `ApiWrite` through the domain layer — the same call
//! the current axum handler makes, minus the HTTP extraction and minus
//! `begin`/`commit`: the executor loop (a later ticket) drives the
//! transaction, and `exec` runs inside one live connection. It is bounded by
//! the `Store` marker, not by the executor's `Txn` trait, so it stays
//! verifiable on both adapters (`MemStore` and `SqlxConn`) per the store-seam
//! contract, with the Postgres behavior as the source of truth.
//!
//! The return is the `Summary` of everything the operation touched, per the
//! write contract in `docs/agents/domain-flow.md`, wrapped in a
//! [`WriteOutcome`]:
//!
//! - operations that already return a `Summary` pass it through;
//! - bare-entity operations are wrapped into a one-entry `Summary` (`+=`);
//! - deletes report their entity as a `None` tombstone: after the cutover,
//!   every non-create mutation returns 204 and the stream frame is the sole
//!   delivery of the change (spec #446 §6.2), so a deleted entity must reach
//!   the client as a tombstone — the stream merge drops it;
//! - a `ShopSubscription` and a `User` are not kinds of the `Summary`, so
//!   subscription writes and onboarding writes report an empty `Summary` —
//!   the stream carries nothing for them, and the HTTP response is their
//!   delivery;
//! - the one write whose result is an operation outcome the `Summary`
//!   cannot carry is `ActivityDescend`: the CSV rows the domain matched and
//!   skipped (the [`DescendReport`]) ride the HTTP response body — the
//!   matched activities' state still rides the frame (the §6.2 deviation,
//!   recorded on issue #446).

use time::OffsetDateTime;

use crate::*;

/// The parsed multipart file of a file-note upload — the raw fields as the
/// handler's parser extracts them, before any fallback. Fallbacks for
/// blank/missing `name` and `filename` are applied by the consumer: [`exec`]
/// for the create route, the domain op itself for the update route.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteFile {
    /// The note's display name; blank or missing.
    pub name: Option<String>,
    /// The file's MIME type.
    pub mime: String,
    /// The uploaded file's name; blank or missing.
    pub filename: Option<String>,
    /// The file's bytes.
    pub data: Vec<u8>,
}

/// One mutating API route, as the per-user executor receives it.
///
/// The variants are the full mutating route table of `backend/axum`
/// (`src/domain/*.rs` plus the two onboarding routes in `src/strava/webhook.rs`),
/// entity-prefixed. The admin routes are deliberately absent: they are
/// admin-triggered, not user writes (spec #446 §3).
#[derive(Clone, Debug, PartialEq)]
pub enum ApiWrite {
    // --- activity (`/api/activ`) ---
    /// `PUT /api/activ/{id}` — change an activity.
    ActivityUpdate { id: ActivityId, activity: Activity },
    /// `DELETE /api/activ/{id}`.
    ActivityDelete { id: ActivityId },
    /// `POST /api/activ/descend` — the raw CSV body; the domain matches the
    /// rows against the user's activities by time.
    ActivityDescend { data: String },
    /// `POST /api/activ/defaultgear` — make this gear the default for the
    /// activity types it serves.
    ActivityDefaultGear { gear: PartId },

    // --- attachment (`/api/part`) ---
    /// `POST /api/part/attach`
    AttachmentAttach {
        part: PartId,
        time: OffsetDateTime,
        gear: PartId,
        hook: PartTypeId,
        all: bool,
    },
    /// `POST /api/part/detach`
    AttachmentDetach {
        part: PartId,
        time: OffsetDateTime,
        all: bool,
    },
    /// `POST /api/part/dispose`
    AttachmentDispose {
        part: PartId,
        time: OffsetDateTime,
        all: bool,
    },
    /// `POST /api/part/recover`
    AttachmentRecover { part: PartId, all: bool },

    // --- part (`/api/part`) ---
    /// `POST /api/part`
    PartCreate {
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        purchase: OffsetDateTime,
    },
    /// `PUT /api/part/{part}`
    PartChange {
        id: PartId,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
    },
    /// `DELETE /api/part/{part}`
    PartDelete { id: PartId },

    // --- partnote (`/api/part`) ---
    /// `POST /api/part/{part}/notes`
    PartNoteCreateText { part: PartId, name: String },
    /// `POST /api/part/{part}/notes/file` — the parsed multipart file;
    /// blank/missing `name` and `filename` fall back in [`exec`], like the
    /// handler's parser does.
    PartNoteCreateFile { part: PartId, file: NoteFile },
    /// `PUT /api/part/notes/{id}`
    PartNoteUpdateText { id: PartNoteId, name: String },
    /// `PUT /api/part/notes/{id}/file` — the parsed multipart file; the
    /// domain op applies its own fallbacks to the stored note.
    PartNoteUpdateFile { id: PartNoteId, file: NoteFile },
    /// `DELETE /api/part/notes/{id}/file`
    PartNoteRemoveFile { id: PartNoteId },
    /// `DELETE /api/part/notes/{id}`
    PartNoteDelete { id: PartNoteId },

    // --- service (`/api/service`) ---
    /// `POST /api/service`
    ServiceCreate {
        part: PartId,
        time: OffsetDateTime,
        name: String,
        notes: String,
        plans: Vec<ServicePlanId>,
    },
    /// `PUT /api/service`
    ServiceUpdate { service: Service },
    /// `DELETE /api/service/{id}`
    ServiceDelete { id: ServiceId },
    /// `POST /api/service/redo`
    ServiceRedo { service: Service },

    // --- serviceplan (`/api/plan`) ---
    /// `POST /api/plan`
    ServicePlanCreate { plan: ServicePlan },
    /// `PUT /api/plan`
    ServicePlanUpdate { plan: ServicePlan },
    /// `DELETE /api/plan/{id}`
    ServicePlanDelete { id: ServicePlanId },

    // --- shop (`/api/shop`) ---
    /// `POST /api/shop`
    ShopCreate {
        name: String,
        description: Option<String>,
        auto_approve: bool,
    },
    /// `PUT /api/shop/{shop}`
    ShopUpdate {
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    },
    /// `DELETE /api/shop/{shop}`
    ShopDelete { id: ShopId },
    /// `POST /api/shop/{shop}/parts`
    ShopRegisterPart { shop: ShopId, part: PartId },
    /// `DELETE /api/shop/{shop}/parts/{part}`
    ShopUnregisterPart { shop: ShopId, part: PartId },
    /// `POST /api/shop/subscriptions`
    ShopSubscriptionCreate {
        shop: ShopId,
        message: Option<String>,
    },
    /// `POST /api/shop/subscriptions/{subscription}/approve`
    ShopSubscriptionApprove {
        id: SubscriptionId,
        message: Option<String>,
    },
    /// `POST /api/shop/subscriptions/{subscription}/reject`
    ShopSubscriptionReject {
        id: SubscriptionId,
        message: Option<String>,
    },
    /// `DELETE /api/shop/subscriptions/{subscription}`
    ShopSubscriptionCancel { id: SubscriptionId },

    // --- user (onboarding, `/strava`) ---
    /// Trigger the initial Strava sync (the query's `time`, 0 = "now"). The
    /// domain part runs in [`exec`]; the variant carries `time` for the
    /// cutover, which builds the sync Event from it.
    UserOnboardingSync { time: i64 },
    /// Postpone the initial sync (no route arguments).
    UserOnboardingPostpone,
}

/// The outcome of one applied [`ApiWrite`]: the `Summary` of everything the
/// operation touched (the SSE frame the executor pushes to the user's
/// streams), plus — for the writes whose result is an operation outcome the
/// `Summary` cannot carry — that outcome.
///
/// Today that is [`ApiWrite::ActivityDescend`] alone: the CSV rows the
/// domain matched and skipped (the [`DescendReport`]) are the handler's
/// response body (`200 + {good, bad}`), not `Summary` state — the matched
/// activities' updated state still rides the frame (the spec #446 §6.2
/// deviation, recorded on issue #446).
#[derive(Clone, Debug, PartialEq)]
pub enum WriteOutcome {
    /// The write's `Summary` (every write except the descend).
    Summary(Summary),
    /// `ActivityDescend`: the `Summary` plus the match report of the CSV
    /// rows.
    Descend {
        summary: Summary,
        report: DescendReport,
    },
}

impl WriteOutcome {
    /// The `Summary` of everything the write touched — the frame the
    /// executor pushes to the user's streams.
    pub fn summary(&self) -> &Summary {
        match self {
            Self::Summary(summary) => summary,
            Self::Descend { summary, .. } => summary,
        }
    }
}

/// Apply one [`ApiWrite`] through the domain layer.
///
/// This is the single entry point the executor loop and the write handlers
/// use to apply an API write. It runs **inside** one live transaction — the
/// executor loop (a later ticket) owns `begin`/`commit`/`rollback` — so it
/// takes the session and the store by mutable reference, exactly like the
/// domain operations it calls, and it never touches the transaction
/// lifecycle itself.
///
/// Each arm is the same call the current axum handler makes, minus the HTTP
/// extraction: path ids are already in their domain types, and the
/// non-parameterized constants the handlers pass (`None` service successor,
/// the `PartNote` created-time, a part's `None` source) are replicated here.
///
/// The `UserOnboarding*` arms do the **domain part only** — the status guard
/// (the same `BadRequest` messages as the webhook handlers) and the status
/// update. The Strava-side sync event is deliberately not part of `exec`:
/// `tb_domain` cannot depend on `tb_strava` (the dependency direction is
/// `domain ← strava`), and the in-memory `MemStore` is not a `StravaStore`.
/// The cutover ticket (issue #457) enqueues a sync `Event` on the user's
/// executor channel for `UserOnboardingSync`.
///
/// Returns the `Summary` of everything the operation touched (see the module
/// docs for how bare-entity results, tombstones, and the non-Summary
/// `ShopSubscription`/`User` outcomes are reported), wrapped in a
/// [`WriteOutcome`] (the descend carries its [`DescendReport`] alongside).
pub async fn exec(
    write: ApiWrite,
    session: &mut impl Session,
    store: &mut impl Store,
) -> TbResult<WriteOutcome> {
    // The domain operations take `&dyn Session`; borrow it once for every arm.
    let session = &*session;

    match write {
        // --- activity ---
        ApiWrite::ActivityUpdate { id, activity } => {
            // The handler's guard: the path id and the body's id must agree.
            if id != activity.id {
                return Err(Error::BadRequest(
                    "ActivityId does not match activity".to_string(),
                ));
            }
            Ok(WriteOutcome::Summary(
                activity.update(session, store).await?,
            ))
        }
        ApiWrite::ActivityDelete { id } => {
            Ok(WriteOutcome::Summary(id.delete(session, store).await?))
        }
        ApiWrite::ActivityDescend { data } => {
            // The op also reports the rows it matched and skipped (the
            // handler's response body): the Summary is what the stream
            // frame carries, the report what the response carries.
            let (summary, report) = Activity::csv2descend(data.as_bytes(), session, store).await?;
            Ok(WriteOutcome::Descend { summary, report })
        }
        ApiWrite::ActivityDefaultGear { gear } => Ok(WriteOutcome::Summary(
            Activity::set_default_part(gear, session, store).await?,
        )),

        // --- attachment ---
        ApiWrite::AttachmentAttach {
            part,
            time,
            gear,
            hook,
            all,
        } => Ok(WriteOutcome::Summary(
            attach_assembly(session, part, time, gear, hook, all, store).await?,
        )),
        ApiWrite::AttachmentDetach { part, time, all } => Ok(WriteOutcome::Summary(
            detach_assembly(session, part, time, all, store).await?,
        )),
        ApiWrite::AttachmentDispose { part, time, all } => Ok(WriteOutcome::Summary(
            dispose_assembly(session, part, time, all, store).await?,
        )),
        ApiWrite::AttachmentRecover { part, all } => Ok(WriteOutcome::Summary(
            recover_assembly(session, part, all, store).await?,
        )),

        // --- part ---
        ApiWrite::PartCreate {
            name,
            vendor,
            model,
            what,
            purchase,
        } => {
            // `source` is a Strava import detail the route does not
            // parameterize: a part created through the API has none.
            let part =
                Part::create(name, vendor, model, what, None, purchase, session, store).await?;
            Ok(WriteOutcome::Summary(one_part(part)))
        }
        ApiWrite::PartChange {
            id,
            name,
            vendor,
            model,
            purchase,
        } => {
            let part = id
                .change(name, vendor, model, purchase, session, store)
                .await?;
            Ok(WriteOutcome::Summary(one_part(part)))
        }
        ApiWrite::PartDelete { id } => {
            // The op returns the id; the part itself is reported as a
            // tombstone so the stream merge drops it (the write contract:
            // everything touched).
            id.delete(session, store).await?;
            let mut summary = Summary::default();
            summary.parts.insert(id, None);
            Ok(WriteOutcome::Summary(summary))
        }

        // --- partnote ---
        ApiWrite::PartNoteCreateText { part, name } => {
            let note = part
                .note_create_text(session, name, OffsetDateTime::now_utc(), store)
                .await?;
            Ok(WriteOutcome::Summary(one_part_note(note)))
        }
        ApiWrite::PartNoteCreateFile { part, file } => {
            // The handler's multipart parsing applies these fallbacks before
            // the domain op; they live here, so the handler only parses.
            let filename = file.filename.unwrap_or_else(|| "file".to_string());
            let name = file.name.unwrap_or_else(|| filename.clone());
            let note = part
                .note_create_file(
                    session,
                    name,
                    file.mime,
                    Some(filename),
                    file.data.len() as i64,
                    file.data,
                    OffsetDateTime::now_utc(),
                    store,
                )
                .await?;
            Ok(WriteOutcome::Summary(one_part_note(note)))
        }
        ApiWrite::PartNoteUpdateText { id, name } => {
            let note = id.update_text(session, name, store).await?;
            Ok(WriteOutcome::Summary(one_part_note(note)))
        }
        ApiWrite::PartNoteUpdateFile { id, file } => {
            let note = id
                .update_file(
                    session,
                    file.name,
                    file.mime,
                    file.filename,
                    file.data.len() as i64,
                    file.data,
                    store,
                )
                .await?;
            Ok(WriteOutcome::Summary(one_part_note(note)))
        }
        ApiWrite::PartNoteRemoveFile { id } => {
            let note = id.remove_file(session, store).await?;
            Ok(WriteOutcome::Summary(one_part_note(note)))
        }
        ApiWrite::PartNoteDelete { id } => {
            // The op returns the id; report the note as a tombstone.
            id.delete(session, store).await?;
            let mut summary = Summary::default();
            summary.part_notes.insert(id, None);
            Ok(WriteOutcome::Summary(summary))
        }

        // --- service ---
        ApiWrite::ServiceCreate {
            part,
            time,
            name,
            notes,
            plans,
        } => {
            // The op itself does not check ownership; the handler runs the
            // check first, and it is a domain op too.
            part.checkuser(session, store).await?;
            // `successor` is a redo of an existing service; the create route
            // always starts a new chain.
            Ok(WriteOutcome::Summary(
                Service::create(part, time, name, notes, None, plans, store).await?,
            ))
        }
        ApiWrite::ServiceUpdate { service } => {
            Ok(WriteOutcome::Summary(service.update(session, store).await?))
        }
        ApiWrite::ServiceDelete { id } => {
            Ok(WriteOutcome::Summary(id.delete(session, store).await?))
        }
        ApiWrite::ServiceRedo { service } => {
            Ok(WriteOutcome::Summary(service.redo(session, store).await?))
        }

        // --- serviceplan ---
        ApiWrite::ServicePlanCreate { plan } => {
            let plan = plan.create(session, store).await?;
            Ok(WriteOutcome::Summary(one_service_plan(plan)))
        }
        ApiWrite::ServicePlanUpdate { plan } => {
            let plan = plan.update(session, store).await?;
            Ok(WriteOutcome::Summary(one_service_plan(plan)))
        }
        ApiWrite::ServicePlanDelete { id } => {
            // The op reports the services it unlinked; the plan itself is
            // deleted as well, so report it as a tombstone too.
            let services = id.delete(session, store).await?;
            let mut summary = Summary::default();
            summary += services;
            summary.plans.insert(id, None);
            Ok(WriteOutcome::Summary(summary))
        }

        // --- shop ---
        ApiWrite::ShopCreate {
            name,
            description,
            auto_approve,
        } => {
            let shop =
                ShopId::create(name, description, auto_approve, session.user_id(), store).await?;
            Ok(WriteOutcome::Summary(one_shop(shop)))
        }
        ApiWrite::ShopUpdate {
            id,
            name,
            description,
            auto_approve,
        } => {
            let id = ShopId::get(id.into(), session.user_id(), store).await?;
            let shop = id
                .update(name, description, auto_approve, session.user_id(), store)
                .await?;
            Ok(WriteOutcome::Summary(one_shop(shop)))
        }
        ApiWrite::ShopDelete { id } => {
            let id = ShopId::get(id.into(), session.user_id(), store).await?;
            id.delete(session.user_id(), store).await?;
            let mut summary = Summary::default();
            summary.shops.insert(id, None);
            Ok(WriteOutcome::Summary(summary))
        }
        ApiWrite::ShopRegisterPart { shop, part } => Ok(WriteOutcome::Summary(
            shop.register_part(part, session, store).await?,
        )),
        ApiWrite::ShopUnregisterPart { shop, part } => Ok(WriteOutcome::Summary(
            shop.unregister_part(part, session, store).await?,
        )),

        // A `ShopSubscription` is not a kind of the `Summary`, so these
        // writes report an empty Summary: the stream carries nothing for
        // them, and the HTTP response is their delivery.
        ApiWrite::ShopSubscriptionCreate { shop, message } => {
            SubscriptionId::create(shop, message, session.user_id(), store).await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }
        ApiWrite::ShopSubscriptionApprove { id, message } => {
            let id = SubscriptionId::get(id.into(), session.user_id(), store).await?;
            id.approve(message, session.user_id(), store).await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }
        ApiWrite::ShopSubscriptionReject { id, message } => {
            let id = SubscriptionId::get(id.into(), session.user_id(), store).await?;
            id.reject(message, session.user_id(), store).await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }
        ApiWrite::ShopSubscriptionCancel { id } => {
            let id = SubscriptionId::get(id.into(), session.user_id(), store).await?;
            id.cancel(session.user_id(), store).await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }

        // --- user (onboarding) ---

        // Domain part only: the status guard, exactly as in the webhook
        // handlers, plus the status update. A `User` is not in any `Summary`,
        // so the result is empty. The Strava-side sync event is the cutover
        // ticket's concern (issue #457) — see the `exec` docs.
        ApiWrite::UserOnboardingSync { .. } => {
            let user = session.user_id().read(store).await?;
            if user.onboarding_status.is_initial_sync_completed() {
                return Err(Error::BadRequest(
                    "Initial sync already triggered".to_string(),
                ));
            }
            store
                .update_onboarding_status(&session.user_id(), OnboardingStatus::Completed)
                .await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }
        ApiWrite::UserOnboardingPostpone => {
            let user = session.user_id().read(store).await?;
            if user.onboarding_status != OnboardingStatus::Pending {
                return Err(Error::BadRequest(
                    "Initial sync already completed or postponed".to_string(),
                ));
            }
            store
                .update_onboarding_status(
                    &session.user_id(),
                    OnboardingStatus::InitialSyncPostponed,
                )
                .await?;
            Ok(WriteOutcome::Summary(Summary::default()))
        }
    }
}

/// A one-entry [`Summary`] upserting a single entity — the body of the three-line
/// shape (`Summary::default()` + `+=` + `Ok`) that every bare-entity arm repeats.
/// One helper per entity kind rather than one generic: the `Summary` `+=` impls
/// are individual, not sealed behind a common bound a generic could name.
macro_rules! one_entry_summary {
    ($name:ident, $entity:ty) => {
        fn $name(entity: $entity) -> Summary {
            let mut summary = Summary::default();
            summary += entity;
            summary
        }
    };
}

one_entry_summary!(one_part, Part);
one_entry_summary!(one_part_note, PartNote);
one_entry_summary!(one_service_plan, ServicePlan);
one_entry_summary!(one_shop, Shop);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        MemStore,
        fixtures::{sample_purchase_date, test_session, test_user},
        part_type_ids::{BIKE, CHAIN},
    };
    use crate::traits::{
        ActivityStore, AttachmentStore, PartNoteStore, PartStore, ShopStore, UserStore,
    };
    use crate::{
        ActTypeId, Activity, ActivityId, Error, MAX_TIME, OnboardingStatus, Part, PartId,
        PartTypeId, ServicePlan, ServicePlanId, SubscriptionStatus, Summary, UserId,
    };
    use std::collections::HashMap;
    use time::macros::datetime;

    /// A fresh hook on a bike that a chain can be attached to (the first hook
    /// of the chain part type, like the in-memory attachment suite does).
    fn chain_hook() -> PartTypeId {
        CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN)
    }

    /// The `Summary` of a write's outcome: every write under test reports
    /// its `Summary` (the descend's `DescendReport` is asserted explicitly
    /// where it is the point).
    fn outcome_summary(outcome: WriteOutcome) -> Summary {
        match outcome {
            WriteOutcome::Summary(summary) => summary,
            WriteOutcome::Descend { summary, .. } => summary,
        }
    }

    // --- Part ---

    #[tokio::test]
    async fn part_create_dispatches() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(
                ApiWrite::PartCreate {
                    name: "New Chain".to_string(),
                    vendor: "Shimano".to_string(),
                    model: "CN-HG62".to_string(),
                    what: CHAIN,
                    purchase: sample_purchase_date(),
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.parts.len(), 1);
        let part = summary.parts.values().flatten().next().unwrap();
        assert_eq!(part.name, "New Chain");
        assert_eq!(part.owner, test_user());
        assert_eq!(part.what, CHAIN);
        Ok(())
    }

    #[tokio::test]
    async fn part_change_dispatches() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let id = PartId::from(13); // "Spare Chain 1" — loose, owned by user 1
        let summary = outcome_summary(
            exec(
                ApiWrite::PartChange {
                    id,
                    name: "Renamed".to_string(),
                    vendor: "New Vendor".to_string(),
                    model: "New Model".to_string(),
                    purchase: sample_purchase_date(),
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.parts[&id].as_ref().unwrap().name, "Renamed");
        Ok(())
    }

    #[tokio::test]
    async fn part_delete_reports_tombstone() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let id = PartId::from(13);
        let summary =
            outcome_summary(exec(ApiWrite::PartDelete { id }, &mut session, &mut store).await?);
        assert_eq!(summary.parts, HashMap::from([(id, None)]));
        assert!(store.partid_get_part(id).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn part_delete_missing_is_not_found() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let err = exec(
            ApiWrite::PartDelete {
                id: PartId::from(9999),
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
        Ok(())
    }

    // --- Attachment ---

    #[tokio::test]
    async fn attach_and_detach_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let bike = Part::create(
            "Bike".to_string(),
            "V".to_string(),
            "M".to_string(),
            BIKE,
            None,
            sample_purchase_date(),
            &session,
            &mut store,
        )
        .await?;
        let chain = Part::create(
            "Chain".to_string(),
            "Shimano".to_string(),
            "CN-M510".to_string(),
            CHAIN,
            None,
            sample_purchase_date(),
            &session,
            &mut store,
        )
        .await?;
        let time = datetime!(2024-01-01 00:00 UTC);

        let summary = outcome_summary(
            exec(
                ApiWrite::AttachmentAttach {
                    part: chain.id,
                    time,
                    gear: bike.id,
                    hook: chain_hook(),
                    all: false,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert!(!summary.parts.is_empty());
        let att = store
            .attachment_get_by_part_and_time(chain.id, time)
            .await?
            .expect("the chain is attached");
        assert_eq!(att.gear, bike.id);
        assert_eq!(att.detached, MAX_TIME);

        let _ = exec(
            ApiWrite::AttachmentDetach {
                part: chain.id,
                time,
                all: false,
            },
            &mut session,
            &mut store,
        )
        .await?;
        assert!(
            store
                .attachment_get_by_part_and_time(chain.id, time)
                .await?
                .is_none()
        );
        Ok(())
    }

    // --- Activity ---

    #[tokio::test]
    async fn activity_update_dispatches() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let mut act = store
            .activity_read_by_id(ActivityId::new(1))
            .await?
            .expect("the fixture activity");
        let id = act.id;
        act.name = "Renamed Ride".to_string();
        let summary = outcome_summary(
            exec(
                ApiWrite::ActivityUpdate { id, activity: act },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(
            summary.activities[&id].as_ref().unwrap().name,
            "Renamed Ride"
        );
        Ok(())
    }

    #[tokio::test]
    async fn activity_update_id_mismatch_is_bad_request() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let act = store
            .activity_read_by_id(ActivityId::new(1))
            .await?
            .expect("the fixture activity");
        // A path id different from the body's: the handler's guard.
        let err = exec(
            ApiWrite::ActivityUpdate {
                id: ActivityId::new(2),
                activity: act,
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
        Ok(())
    }

    #[tokio::test]
    async fn activity_delete_reports_tombstone() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let id = ActivityId::new(3);
        let summary =
            outcome_summary(exec(ApiWrite::ActivityDelete { id }, &mut session, &mut store).await?);
        assert_eq!(summary.activities, HashMap::from([(id, None)]));
        assert!(store.activity_read_by_id(id).await?.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn activity_delete_missing_is_not_found() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let err = exec(
            ApiWrite::ActivityDelete {
                id: ActivityId::new(999_999),
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
        Ok(())
    }

    #[tokio::test]
    async fn activity_descend_updates_matched_activity() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        // A row matching fixture activity 1 (start 2023-05-18 22:13:20Z,
        // matched by local minute) with a descend value.
        let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n";
        let outcome = exec(
            ApiWrite::ActivityDescend {
                data: csv.to_string(),
            },
            &mut session,
            &mut store,
        )
        .await?;
        let WriteOutcome::Descend { summary, report } = outcome else {
            panic!("the descend write carries its match report");
        };
        assert_eq!(
            summary.activities[&ActivityId::new(1)]
                .as_ref()
                .unwrap()
                .descend,
            Some(900)
        );
        // The dispatch carries the match report (the handler's response
        // body), not just the Summary.
        assert_eq!(
            report,
            DescendReport {
                good: vec!["Morning Ride at 2023-05-18 22:13:20".to_string()],
                bad: vec![],
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn activity_default_gear_assigns_null_gear_activities() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let act = Activity {
            id: ActivityId::new(100),
            user_id: test_user(),
            what: ActTypeId::from(1),
            name: "No Gear".to_string(),
            start: datetime!(2024-06-01 10:00 UTC),
            duration: 3600,
            time: Some(3500),
            distance: Some(50000),
            climb: Some(500),
            descend: None,
            energy: Some(1000),
            gear: None,
            device_name: None,
            external_id: None,
        };
        store.activity_create(act.clone()).await?;

        let summary = outcome_summary(
            exec(
                ApiWrite::ActivityDefaultGear {
                    gear: PartId::from(1), // "Main Bike" in the fixture
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        let stored = store
            .activity_read_by_id(act.id)
            .await?
            .expect("the activity still exists");
        assert_eq!(stored.gear, Some(PartId::from(1)));
        assert!(
            summary
                .activities
                .values()
                .flatten()
                .any(|a| a.id == act.id)
        );
        Ok(())
    }

    // --- PartNote ---

    #[tokio::test]
    async fn partnote_create_text_and_delete_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let part = PartId::from(13);
        let summary = outcome_summary(
            exec(
                ApiWrite::PartNoteCreateText {
                    part,
                    name: "Check the tension".to_string(),
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.part_notes.len(), 1);
        let note = summary.part_notes.values().flatten().next().unwrap();
        assert_eq!(note.name, "Check the tension");
        assert_eq!(note.part, part);
        assert!(!note.has_file());

        let summary = outcome_summary(
            exec(
                ApiWrite::PartNoteDelete { id: note.id },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.part_notes, HashMap::from([(note.id, None)]));
        assert!(store.partnote_get(note.id).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn partnote_blank_name_is_bad_request() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let err = exec(
            ApiWrite::PartNoteCreateText {
                part: PartId::from(13),
                name: "   ".to_string(),
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::BadRequest(_)));
        Ok(())
    }

    // --- Service ---

    #[tokio::test]
    async fn service_create_and_delete_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let time = datetime!(2024-06-15 10:00 UTC);
        let summary = outcome_summary(
            exec(
                ApiWrite::ServiceCreate {
                    part: PartId::from(13),
                    time,
                    name: "Chain Service".to_string(),
                    notes: "Old chain".to_string(),
                    plans: vec![],
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.services.len(), 1);
        assert_eq!(summary.usages.len(), 1);
        let service = summary.services.values().flatten().next().unwrap();
        assert_eq!(service.name, "Chain Service");

        let summary = outcome_summary(
            exec(
                ApiWrite::ServiceDelete { id: service.id },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.services, HashMap::from([(service.id, None)]));
        Ok(())
    }

    #[tokio::test]
    async fn service_create_on_foreign_part_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        // Part 1 is owned by user 1; user 2 may not service it.
        let mut session = crate::test_support::TestSession::new(UserId::from(2));
        let err = exec(
            ApiWrite::ServiceCreate {
                part: PartId::from(1),
                time: datetime!(2024-06-15 10:00 UTC),
                name: "Service".to_string(),
                notes: "".to_string(),
                plans: vec![],
            },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::Forbidden(_)));
        Ok(())
    }

    // --- ServicePlan ---

    #[tokio::test]
    async fn serviceplan_create_and_delete_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let plan = ServicePlan {
            id: ServicePlanId::from(uuid::Uuid::now_v7()),
            part: Some(PartId::from(13)),
            what: CHAIN,
            hook: None,
            name: "Chain Every 1000km".to_string(),
            days: None,
            hours: None,
            km: Some(1000),
            climb: None,
            descend: None,
            rides: None,
            uid: None,
            energy: None,
        };
        let summary = outcome_summary(
            exec(
                ApiWrite::ServicePlanCreate { plan },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.plans.len(), 1);
        let plan_id = *summary.plans.keys().next().unwrap();

        let summary = outcome_summary(
            exec(
                ApiWrite::ServicePlanDelete { id: plan_id },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.plans, HashMap::from([(plan_id, None)]));
        Ok(())
    }

    // --- Shop ---

    #[tokio::test]
    async fn shop_create_dispatches() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(
                ApiWrite::ShopCreate {
                    name: "Workshop".to_string(),
                    description: None,
                    auto_approve: true,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.shops.len(), 1);
        let shop = summary.shops.values().flatten().next().unwrap();
        assert_eq!(shop.name, "Workshop");
        assert_eq!(shop.owner, test_user());
        Ok(())
    }

    #[tokio::test]
    async fn shop_register_and_unregister_part_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(
                ApiWrite::ShopCreate {
                    name: "Workshop".to_string(),
                    description: None,
                    auto_approve: true,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        let shop_id = *summary.shops.keys().next().unwrap();
        // The route's checkuser requires an (owner) subscription, so create
        // one first — auto-approve activates it.
        exec(
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id,
                message: None,
            },
            &mut session,
            &mut store,
        )
        .await?;
        let part = PartId::from(13); // loose spare, owned by user 1

        let summary = outcome_summary(
            exec(
                ApiWrite::ShopRegisterPart {
                    shop: shop_id,
                    part,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.parts[&part].as_ref().unwrap().shop, Some(shop_id));

        let summary = outcome_summary(
            exec(
                ApiWrite::ShopUnregisterPart {
                    shop: shop_id,
                    part,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary.parts[&part].as_ref().unwrap().shop, None);
        Ok(())
    }

    #[tokio::test]
    async fn shop_subscription_create_and_cancel_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(
                ApiWrite::ShopCreate {
                    name: "Workshop".to_string(),
                    description: None,
                    auto_approve: true,
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        let shop_id = *summary.shops.keys().next().unwrap();

        // A subscription is not a Summary kind: the write reports an empty
        // Summary, the side effect lands in the store.
        let summary = outcome_summary(
            exec(
                ApiWrite::ShopSubscriptionCreate {
                    shop: shop_id,
                    message: Some("Please approve".to_string()),
                },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary, Summary::default());
        let sub = store
            .subscription_find_active(shop_id, test_user())
            .await?
            .expect("auto-approve activates the subscription");
        assert_eq!(sub.status, SubscriptionStatus::Active);

        let summary = outcome_summary(
            exec(
                ApiWrite::ShopSubscriptionCancel { id: sub.id },
                &mut session,
                &mut store,
            )
            .await?,
        );
        assert_eq!(summary, Summary::default());
        assert!(store.subscription_get(sub.id).await.is_err());
        Ok(())
    }

    // --- User (onboarding) ---

    #[tokio::test]
    async fn user_onboarding_sync_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(
                ApiWrite::UserOnboardingSync { time: 0 },
                &mut session,
                &mut store,
            )
            .await?,
        );
        // A User is not in any Summary.
        assert_eq!(summary, Summary::default());
        let user = UserStore::get(&mut store, UserId::from(1)).await?;
        assert_eq!(user.onboarding_status, OnboardingStatus::Completed);

        // The handler's guard: a second sync is rejected with the same message.
        let err = exec(
            ApiWrite::UserOnboardingSync { time: 0 },
            &mut session,
            &mut store,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            &err,
            Error::BadRequest(msg) if msg == "Initial sync already triggered"
        ));
        Ok(())
    }

    #[tokio::test]
    async fn user_onboarding_postpone_dispatch() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let mut session = test_session();
        let summary = outcome_summary(
            exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store).await?,
        );
        assert_eq!(summary, Summary::default());
        let user = UserStore::get(&mut store, UserId::from(1)).await?;
        assert_eq!(
            user.onboarding_status,
            OnboardingStatus::InitialSyncPostponed
        );

        // The handler's guard: not pending anymore.
        let err = exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store)
            .await
            .unwrap_err();
        assert!(matches!(
            &err,
            Error::BadRequest(msg)
                if msg == "Initial sync already completed or postponed"
        ));
        Ok(())
    }
}
