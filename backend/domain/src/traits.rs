#![allow(clippy::too_many_arguments)]
mod part;
pub use part::*;

mod user;
pub use user::*;

mod shop;
pub use shop::*;

mod activity;
pub use activity::*;

mod attachment;
pub use attachment::*;

mod partnote;
pub use partnote::*;

mod usage;
pub use usage::*;

mod service;
pub use service::*;

mod serviceplan;
pub use serviceplan::*;

use std::marker::PhantomData;
use std::ops::AddAssign;

use crate::{
    ActTypeId, Activity, ActivityId, ApiWrite, Error, Part, PartId, PartTypeId, ShopId, Summary,
    TbResult, Usage, UserId, WriteOutcome,
};
use time::OffsetDateTime;

/// A marker trait naming a complete store: one that implements all nine
/// sub-traits. It carries no methods — the transaction lifecycle
/// (`begin`/`commit`/`rollback`) is inherent on the concrete store types and
/// is driven only by the web layer, the `tb_exec` executor loop, and the
/// tests, never by the domain. The executor loop names that same lifecycle
/// through the `Txn` and `TxnSource` traits in `tb_exec` (ADR-0005,
/// executable spec #446 §4.1).
pub trait Store:
    Send
    + PartStore
    + UserStore
    + ShopStore
    + ActivityStore
    + AttachmentStore
    + PartNoteStore
    + UsageStore
    + ServiceStore
    + ServicePlanStore
{
}

/// A trait that represents a session.
pub trait Session: Send + Sync {
    fn user_id(&self) -> UserId;
    fn shop(&self) -> Option<ShopId>;
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()>;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        self.user_id().check_owner(owner, error)
    }
}

/// The entity-operation abstraction that `tb_strava` and `tb_exec` drive
/// (spec #525, ADR-0006).
///
/// An **entity-operation** abstraction, not a store-method abstraction: the
/// associated types, the operations the consumers drive, and the
/// constructors they build values with — not the 78 `Store` sub-trait
/// methods. The concrete domain ([`ConcreteDomain`]) and the in-crate
/// minimal test domains (issue #529 onward) both satisfy it; a generic
/// consumer names only `D::` types and never a `Store` sub-trait.
///
/// Two non-obvious shapes, both forced by stable Rust (no associated
/// traits):
///
/// - `Store` is an opaque *type*. The concrete impl is generic over it
///   ([`ConcreteDomain<St>`]) because the domain operations are generic
///   over the store (`&mut impl <sub-trait>`), and a trait impl cannot be
///   generic. A `dyn Store` cannot serve: the ops' sub-trait bounds are
///   sized, and a trait object does not implement its supertraits.
/// - `Session` is `?Sized`: the concrete domain sets it to `dyn Session`.
///   The session type varies per caller (the web layer's session, the
///   executor's `S: StravaSession`), and the trait object is the one type
///   every caller can coerce to.
#[async_trait::async_trait]
pub trait Domain {
    /// The domain's error type.
    type Error;
    /// The operation-result map. Bounded exactly by what the executor needs
    /// (`broadcast::Sender`'s `Clone`, summary merging's `AddAssign`,
    /// `Summary::default()`) — the concrete [`Summary`] satisfies all three.
    type Summary: Default + Clone + AddAssign<Self::Summary>;
    /// The session the operations take (`dyn Session` in the concrete
    /// domain; a concrete type in the minimal test domains).
    type Session: Send + Sync + ?Sized;
    /// The store the operations take; opaque to the generic consumer.
    /// `Send` so the boxed async futures can cross threads.
    type Store: Send;
    /// The API write the executor dispatches.
    type ApiWrite;
    /// The dispatch outcome.
    type WriteOutcome;
    /// The entity types the consumers construct and name.
    type Activity;
    type ActivityId;
    type ActTypeId;
    type PartId;
    type PartTypeId;
    type UserId;
    type Part;
    type Usage;

    /// `Activity::upsert`: create or replace the activity, accounting its
    /// gear's parts.
    async fn upsert_activity(
        activity: Self::Activity,
        user: &Self::Session,
        store: &mut Self::Store,
    ) -> Result<Self::Summary, Self::Error>;

    /// `ActivityId::read`: the activity with the id, authorization-checked.
    async fn read_activity(
        id: Self::ActivityId,
        session: &Self::Session,
        store: &mut Self::Store,
    ) -> Result<Self::Activity, Self::Error>;

    /// `ActivityId::delete`: delete the activity and reverse its usage.
    async fn delete_activity(
        id: Self::ActivityId,
        session: &Self::Session,
        store: &mut Self::Store,
    ) -> Result<Self::Summary, Self::Error>;

    /// `Part::create`: a new part for the session's user/shop.
    async fn create_part(
        name: String,
        vendor: String,
        model: String,
        what: Self::PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: &Self::Session,
        store: &mut Self::Store,
    ) -> Result<Self::Part, Self::Error>;

    /// `Part::by_source`: the part carrying the Strava source, if any.
    async fn part_by_source(
        source: &str,
        store: &mut Self::Store,
    ) -> Result<Option<Self::Part>, Self::Error>;

    /// `exec`: the `ApiWrite` dispatch — the executor's write lane.
    async fn exec(
        write: Self::ApiWrite,
        session: &mut Self::Session,
        store: &mut Self::Store,
    ) -> Result<Self::WriteOutcome, Self::Error>;

    /// The 14-field activity constructor (`tb_strava`'s `into_activity`).
    fn make_activity(
        id: Self::ActivityId,
        user_id: Self::UserId,
        what: Self::ActTypeId,
        name: String,
        start: OffsetDateTime,
        duration: i32,
        time: Option<i32>,
        distance: Option<i32>,
        climb: Option<i32>,
        descend: Option<i32>,
        energy: Option<i32>,
        gear: Option<Self::PartId>,
        device_name: Option<String>,
        external_id: Option<String>,
    ) -> Self::Activity;

    /// `ActivityId::new`.
    fn make_activity_id(id: i64) -> Self::ActivityId;

    /// `PartId::from`.
    fn make_part_id(id: i32) -> Self::PartId;
}

/// The concrete domain: `tb_domain`'s entity operations behind the
/// [`Domain`] trait (spec #525, ADR-0006).
///
/// Parameterized by the store type — the domain operations are generic over
/// the store, and a trait impl cannot be generic (stable Rust has no
/// associated traits), so the genericity lives here. Each operation maps
/// 1:1 to the existing domain operation; nothing is re-implemented. Named
/// only where a `Domain` impl is needed (the composition root, the
/// executor's bounds).
pub struct ConcreteDomain<St: Store + Send>(PhantomData<St>);

#[async_trait::async_trait]
impl<St: Store + Send> Domain for ConcreteDomain<St> {
    type Error = Error;
    type Summary = Summary;
    type Session = dyn Session;
    type Store = St;
    type ApiWrite = ApiWrite;
    type WriteOutcome = WriteOutcome;
    type Activity = Activity;
    type ActivityId = ActivityId;
    type ActTypeId = ActTypeId;
    type PartId = PartId;
    type PartTypeId = PartTypeId;
    type UserId = UserId;
    type Part = Part;
    type Usage = Usage;

    async fn upsert_activity(
        activity: Activity,
        user: &(dyn Session + 'static),
        store: &mut St,
    ) -> TbResult<Summary> {
        activity.upsert(user, store).await
    }

    async fn read_activity(
        id: ActivityId,
        session: &(dyn Session + 'static),
        store: &mut St,
    ) -> TbResult<Activity> {
        id.read(session, store).await
    }

    async fn delete_activity(
        id: ActivityId,
        session: &(dyn Session + 'static),
        store: &mut St,
    ) -> TbResult<Summary> {
        id.delete(session, store).await
    }

    async fn create_part(
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: &(dyn Session + 'static),
        store: &mut St,
    ) -> TbResult<Part> {
        Part::create(name, vendor, model, what, source, purchase, user, store).await
    }

    async fn part_by_source(source: &str, store: &mut St) -> TbResult<Option<Part>> {
        Part::by_source(source, store).await
    }

    async fn exec(
        write: ApiWrite,
        session: &mut (dyn Session + 'static),
        store: &mut St,
    ) -> TbResult<WriteOutcome> {
        crate::apiwrite::exec_dyn(write, &*session, store).await
    }

    fn make_activity(
        id: ActivityId,
        user_id: UserId,
        what: ActTypeId,
        name: String,
        start: OffsetDateTime,
        duration: i32,
        time: Option<i32>,
        distance: Option<i32>,
        climb: Option<i32>,
        descend: Option<i32>,
        energy: Option<i32>,
        gear: Option<PartId>,
        device_name: Option<String>,
        external_id: Option<String>,
    ) -> Activity {
        Activity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
            device_name,
            external_id,
        }
    }

    fn make_activity_id(id: i64) -> ActivityId {
        ActivityId::new(id)
    }

    fn make_part_id(id: i32) -> PartId {
        PartId::from(id)
    }
}

#[cfg(test)]
mod traits_tests;
