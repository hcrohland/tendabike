//! Tests for the `Domain` trait and its concrete impl (issue #528).
//!
//! The concrete domain (`ConcreteDomain<MemStore>`) must satisfy `Domain`,
//! and driven **through the trait** it must behave exactly like the domain
//! operations it maps to — the generic-consumer pattern `tb_strava`/`tb_exec`
//! will use (spec #525, ADR-0006).

use super::*;
use crate::test_support::{
    MemStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    only_tombstone,
    part_type_ids::{BIKE, CHAIN},
    sole_live,
};
use crate::{ActTypeId, Activity, ActivityId, ApiWrite, PartId};
use time::{OffsetDateTime, macros::datetime};

/// The concrete domain over the in-memory store.
type DomainOverMem = ConcreteDomain<MemStore>;

/// Compile-time proof: the concrete domain satisfies `Domain` for a store.
fn assert_domain<D: Domain>() {}

#[test]
fn concrete_domain_satisfies_domain() {
    assert_domain::<DomainOverMem>();
}

// --- Generic consumer helpers.
//
// Every operation below is driven *through the `Domain` trait*, naming only
// `D::` types — the exact pattern the generic `tb_strava`/`tb_exec` seams will
// use (issue #529), so these tests also pin the trait's call surface.

async fn upsert_activity<D: Domain>(
    activity: D::Activity,
    user: &impl Session,
    store: &mut D::Store,
) -> D::Summary
where
    D::Error: std::fmt::Debug,
{
    D::upsert_activity(activity, user, store).await.unwrap()
}

async fn read_activity<D: Domain>(
    id: D::ActivityId,
    session: &impl Session,
    store: &mut D::Store,
) -> D::Activity
where
    D::Error: std::fmt::Debug,
{
    D::read_activity(id, session, store).await.unwrap()
}

async fn delete_activity<D: Domain>(
    id: D::ActivityId,
    session: &impl Session,
    store: &mut D::Store,
) -> D::Summary
where
    D::Error: std::fmt::Debug,
{
    D::delete_activity(id, session, store).await.unwrap()
}

async fn create_part<D: Domain>(
    name: String,
    vendor: String,
    model: String,
    what: D::PartTypeId,
    source: Option<String>,
    purchase: OffsetDateTime,
    user: &impl Session,
    store: &mut D::Store,
) -> D::Part
where
    D::Error: std::fmt::Debug,
{
    D::create_part(name, vendor, model, what, source, purchase, user, store)
        .await
        .unwrap()
}

async fn part_by_source<D: Domain>(source: &str, store: &mut D::Store) -> Option<D::Part>
where
    D::Error: std::fmt::Debug,
{
    D::part_by_source(source, store).await.unwrap()
}

async fn exec_write<D: Domain>(
    write: D::ApiWrite,
    session: &mut impl Session,
    store: &mut D::Store,
) -> D::WriteOutcome
where
    D::Error: std::fmt::Debug,
{
    D::exec(write, session, store).await.unwrap()
}

/// A 14-field activity built through the trait's constructor, as
/// `tb_strava`'s `into_activity` does.
fn sample_activity(id: i64, name: &str) -> Activity {
    DomainOverMem::make_activity(
        DomainOverMem::make_activity_id(id),
        test_user(),
        ActTypeId::from(1),
        name.to_string(),
        datetime!(2024-05-01 10:30 UTC),
        3600,
        Some(3500),
        Some(15200),
        Some(120),
        None,
        Some(2050),
        None,
        Some("Garmin Edge".to_string()),
        Some("garmin-123".to_string()),
    )
}

#[test]
fn constructors_build_the_entity_values() {
    let id = DomainOverMem::make_activity_id(42);
    assert_eq!(id, ActivityId::new(42));

    let gear = DomainOverMem::make_part_id(7);
    assert_eq!(gear, PartId::from(7));

    let act = sample_activity(42, "Morning Ride");
    assert_eq!(act.id, id);
    assert_eq!(act.user_id, test_user());
    assert_eq!(act.what, ActTypeId::from(1));
    assert_eq!(act.name, "Morning Ride");
    assert_eq!(act.start, datetime!(2024-05-01 10:30 UTC));
    assert_eq!(act.duration, 3600);
    assert_eq!(act.time, Some(3500));
    assert_eq!(act.distance, Some(15200));
    assert_eq!(act.climb, Some(120));
    assert_eq!(act.descend, None);
    assert_eq!(act.energy, Some(2050));
    assert_eq!(act.gear, None);
    assert_eq!(act.device_name, Some("Garmin Edge".to_string()));
    assert_eq!(act.external_id, Some("garmin-123".to_string()));
}

#[tokio::test]
async fn upsert_activity_drives_the_domain_op() {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let act = sample_activity(9001, "Synced Ride");

    let summary = upsert_activity::<DomainOverMem>(act.clone(), &session, &mut store).await;
    let live = sole_live(&summary.activities);
    assert_eq!(live.id, act.id);
    assert_eq!(live.user_id, test_user());
    assert_eq!(live.name, "Synced Ride");
}

#[tokio::test]
async fn read_activity_reads_the_upserted_activity() {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let act = sample_activity(9002, "Read Back");
    upsert_activity::<DomainOverMem>(act.clone(), &session, &mut store).await;

    let read = read_activity::<DomainOverMem>(act.id, &session, &mut store).await;
    assert_eq!(read.id, act.id);
    assert_eq!(read.name, "Read Back");
    assert_eq!(read.start, act.start);
}

#[tokio::test]
async fn delete_activity_tombstones_it() {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let act = sample_activity(9003, "To Delete");
    upsert_activity::<DomainOverMem>(act, &session, &mut store).await;

    let summary =
        delete_activity::<DomainOverMem>(ActivityId::new(9003), &session, &mut store).await;
    assert_eq!(summary.activities, only_tombstone(ActivityId::new(9003)));
}

#[tokio::test]
async fn create_part_creates_the_part() {
    let mut store = MemStore::prepopulated();
    let session = test_session();

    let part = create_part::<DomainOverMem>(
        "New Bike".to_string(),
        "Trek".to_string(),
        "Marlin 7".to_string(),
        BIKE,
        Some("b12345".to_string()),
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await;
    assert_eq!(part.name, "New Bike");
    assert_eq!(part.vendor, "Trek");
    assert_eq!(part.model, "Marlin 7");
    assert_eq!(part.what, BIKE);
    assert_eq!(part.source, Some("b12345".to_string()));
    assert_eq!(part.owner, test_user());
}

#[tokio::test]
async fn part_by_source_finds_the_part() {
    let mut store = MemStore::prepopulated();
    let session = test_session();

    let part = create_part::<DomainOverMem>(
        "Strava Bike".to_string(),
        "Canyon".to_string(),
        "Endural".to_string(),
        BIKE,
        Some("g777".to_string()),
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await;

    let found = part_by_source::<DomainOverMem>("g777", &mut store).await;
    assert_eq!(found.as_ref().unwrap().id, part.id);
    assert!(
        part_by_source::<DomainOverMem>("no-such-source", &mut store)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn exec_dispatches_the_write() {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();

    let outcome = exec_write::<DomainOverMem>(
        ApiWrite::PartCreate {
            name: "Exec Chain".to_string(),
            vendor: "Shimano".to_string(),
            model: "CN-HG62".to_string(),
            what: CHAIN,
            purchase: sample_purchase_date(),
        },
        &mut session,
        &mut store,
    )
    .await;
    let part = sole_live(&outcome.summary().parts);
    assert_eq!(part.name, "Exec Chain");
    assert_eq!(part.what, CHAIN);
}
