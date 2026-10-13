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
    ActTypeId, Activity, ActivityId, ApiWrite, Error, Part, PartId, PartTypeId, ShopId, Stat,
    Summary, TbResult, Usage, UserId, WriteOutcome,
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

/// The class of a domain error, as the Strava event classifier (spec #446
/// §4.6) needs it: the classifier lives in `tb_strava`, which is generic
/// over the domain and cannot name the concrete [`Error`]'s variants
/// (issue #529).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StravaFailure {
    /// `TryAgain`: Strava rate limited — the global `Stop` is queued (the
    /// message is the rate-limit reason).
    TryAgain(String),
    /// `NotAuth`: the Strava auth is dead — the user is disabled (the
    /// message is the auth failure).
    NotAuth(String),
    /// `DatabaseFailure`: the transaction rolls back, the loop backs off.
    DatabaseFailure,
    /// `NotFound`: the object is gone — a delete of a missing activity is
    /// not an error.
    NotFound,
    /// Any other (permanent) failure — the event is dropped.
    Other,
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
/// - The operations take the session as `&impl Session` (not an associated
///   type): a caller's session type is its own (the web layer's session,
///   the executor's `S`), and the operations are generic over it — a
///   `type Session` associated type would force the caller to name
///   `D::Session`, which stable Rust cannot express as a bound.
#[async_trait::async_trait]
pub trait Domain {
    /// The domain's error type. `Debug` + `Display` for the executor's and
    /// the strava lane's logging; `Send` (it rides the executor's channel
    /// oneshot and its loop's return); `Into<anyhow::Error>` so the
    /// executor's transaction-lifecycle failures (a failed
    /// `begin`/`commit`/`rollback`) can be re-wrapped as a generic
    /// `AnyFailure` exactly as the concrete loop does.
    type Error: std::fmt::Debug + std::fmt::Display + Send + Into<anyhow::Error>;
    /// The operation-result map. Bounded exactly by what the executor needs
    /// (`broadcast::Sender`'s `Clone`, summary merging's `AddAssign`,
    /// `Summary::default()`, `Send` — the frame rides the executor's
    /// broadcast) — the concrete [`Summary`] satisfies all four.
    type Summary: Default + Clone + Send + AddAssign<Self::Summary>;
    /// The store the operations take; opaque to the generic consumer.
    /// `Send` so the boxed async futures can cross threads.
    type Store: Send;
    /// The API write the executor dispatches. `Debug` for the executor's
    /// `ApiWriteRequest` logging; `PartialEq` (the executor's `Message`
    /// compares its writes in tests); `Clone` (the executor's
    /// channel).
    type ApiWrite: std::fmt::Debug + Send + Clone + std::cmp::PartialEq;
    /// The dispatch outcome. `Send` (it rides the executor's channel
    /// oneshot).
    type WriteOutcome: Send;
    /// The entity types the consumers construct and name.
    type Activity;
    /// `Display`: the strava activity lane formats the id into the Strava
    /// URL (issue #530).
    type ActivityId: std::fmt::Display;
    /// `From<i32>`: the strava-side type maps are string → code → id
    /// (`tb_strava`'s `get_type`/`StravaGear::what`), and the code is a
    /// plain `i32` (issue #529).
    type ActTypeId: From<i32>;
    type PartId;
    /// `From<i32>`: the strava gear map is frame-type → code → id
    /// (`tb_strava`'s `StravaGear::what`), and the code is a plain `i32`
    /// (issue #529).
    type PartTypeId: From<i32>;
    /// `Display`: the strava lane logs it (the `disable`/`deauthorize`
    /// messages); `Copy`: the session's `tb_id` is passed by value to
    /// several lane ops at once (issue #530).
    type UserId: std::fmt::Display + Copy;
    type Part;
    type Usage;

    /// `Activity::upsert`: create or replace the activity, accounting its
    /// gear's parts.
    async fn upsert_activity(
        activity: Self::Activity,
        user: &impl Session,
        store: &mut Self::Store,
    ) -> Result<Self::Summary, Self::Error>;

    /// `ActivityId::read`: the activity with the id, authorization-checked.
    async fn read_activity(
        id: Self::ActivityId,
        session: &impl Session,
        store: &mut Self::Store,
    ) -> Result<Self::Activity, Self::Error>;

    /// `ActivityId::delete`: delete the activity and reverse its usage.
    async fn delete_activity(
        id: Self::ActivityId,
        session: &impl Session,
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
        user: &impl Session,
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
        session: &mut impl Session,
        store: &mut Self::Store,
    ) -> Result<Self::WriteOutcome, Self::Error>;

    /// `Summary::is_empty` (ADR-0005 §4.5): the executor uses this to decide
    /// the stream-frame push (an empty `Summary` pushes nothing). Exposed
    /// because the executor is generic over the domain and cannot name the
    /// concrete [`Summary`] (issue #529).
    fn summary_is_empty(summary: &Self::Summary) -> bool;

    /// `WriteOutcome::summary`: the `Summary` of everything the write touched
    /// — the frame the executor pushes to the user's streams. Exposed for the
    /// same reason as [`summary_is_empty`] (issue #529).
    fn write_outcome_summary(outcome: &Self::WriteOutcome) -> &Self::Summary;

    /// `Error::AnyFailure`: wrap an `anyhow::Error` as the domain's generic
    /// failure — the executor's transaction-lifecycle errors (a failed
    /// `begin`/`commit`/`rollback`) are re-wrapped this way (issue #529).
    fn any_failure(err: anyhow::Error) -> Self::Error;

    /// The Strava event classifier's view of a domain error (spec #446
    /// §4.6): the classifier (`tb_strava`'s `fail_event`/`sync`) is generic
    /// over the domain and cannot name the concrete [`Error`]'s variants
    /// (issue #529).
    fn strava_failure(err: &Self::Error) -> StravaFailure;

    /// `Error::BadRequest`: a bad request — the strava-side constructor the
    /// generic lane needs (an unsupported activity type, issue #529).
    fn bad_request(msg: String) -> Self::Error;

    /// `Part.id`: the part's id — the strava gear lane maps a part to its
    /// id (the part is opaque to the generic consumer, issue #529).
    fn part_id(part: &Self::Part) -> Self::PartId;

    /// `Activity.id`: the activity's id — the strava activity lane maps an
    /// activity to its id for the Strava URL (issue #530).
    fn activity_id(activity: &Self::Activity) -> Self::ActivityId;

    /// `Error::NotFound`: the object is gone — the strava-side constructor the
    /// generic lane needs (issue #530).
    fn not_found(msg: String) -> Self::Error;

    /// `PartId::part`: the part with the id, authorization-checked — the
    /// strava gear-URL lane (issue #530). Named with the concrete [`Part`]:
    /// the lane reads its `source` field, which an opaque associated type
    /// cannot expose.
    async fn read_part(
        id: Self::PartId,
        session: &impl Session,
        store: &mut Self::Store,
    ) -> Result<Part, Self::Error>;

    /// `UserId::create`: a new user (the strava auth lane, issue #530).
    async fn create_user(
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut Self::Store,
    ) -> Result<UserId, Self::Error>;

    /// `UserId::update`: the user's name and avatar (issue #530).
    async fn update_user(
        id: UserId,
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut Self::Store,
    ) -> Result<UserId, Self::Error>;

    /// `UserId::delete`: the user plus everything they own (issue #530).
    /// Named with [`Self::UserId`]: the lane receives the id from the
    /// session's `tb_id`.
    async fn delete_user(id: Self::UserId, store: &mut Self::Store) -> Result<(), Self::Error>;

    /// `UserId::get_stat`: the user's stats (the strava stats lane, issue
    /// #530). Named with the concrete [`Stat`]: the lane serializes it
    /// straight into its response.
    async fn read_stats(id: UserId, store: &mut Self::Store) -> Result<Stat, Self::Error>;

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
        user: &impl Session,
        store: &mut St,
    ) -> TbResult<Summary> {
        activity.upsert(user, store).await
    }

    async fn read_activity(
        id: ActivityId,
        session: &impl Session,
        store: &mut St,
    ) -> TbResult<Activity> {
        id.read(session, store).await
    }

    async fn delete_activity(
        id: ActivityId,
        session: &impl Session,
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
        user: &impl Session,
        store: &mut St,
    ) -> TbResult<Part> {
        Part::create(name, vendor, model, what, source, purchase, user, store).await
    }

    async fn part_by_source(source: &str, store: &mut St) -> TbResult<Option<Part>> {
        Part::by_source(source, store).await
    }

    async fn exec(
        write: ApiWrite,
        session: &mut impl Session,
        store: &mut St,
    ) -> TbResult<WriteOutcome> {
        crate::apiwrite::exec_dyn(write, &*session, store).await
    }

    fn summary_is_empty(summary: &Summary) -> bool {
        summary.is_empty()
    }

    fn write_outcome_summary(outcome: &WriteOutcome) -> &Summary {
        outcome.summary()
    }

    fn any_failure(err: anyhow::Error) -> Error {
        Error::AnyFailure(err)
    }

    fn strava_failure(err: &Error) -> StravaFailure {
        match err {
            Error::TryAgain(msg) => StravaFailure::TryAgain(msg.to_string()),
            Error::NotAuth(msg) => StravaFailure::NotAuth(msg.clone()),
            Error::DatabaseFailure(_) => StravaFailure::DatabaseFailure,
            Error::NotFound(_) => StravaFailure::NotFound,
            _ => StravaFailure::Other,
        }
    }

    fn bad_request(msg: String) -> Error {
        Error::BadRequest(msg)
    }

    fn part_id(part: &Part) -> PartId {
        part.id
    }

    fn activity_id(activity: &Activity) -> ActivityId {
        activity.id
    }

    fn not_found(msg: String) -> Error {
        Error::NotFound(msg)
    }

    async fn read_part(id: PartId, session: &impl Session, store: &mut St) -> TbResult<Part> {
        id.part(session, store).await
    }

    async fn create_user(
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut St,
    ) -> TbResult<UserId> {
        UserId::create(&firstname, &lastname, avatar, store).await
    }

    async fn update_user(
        id: UserId,
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut St,
    ) -> TbResult<UserId> {
        id.update(&firstname, &lastname, avatar, store).await
    }

    async fn delete_user(id: UserId, store: &mut St) -> TbResult<()> {
        id.delete(store).await
    }
    async fn read_stats(id: UserId, store: &mut St) -> TbResult<Stat> {
        id.get_stat(store).await
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
