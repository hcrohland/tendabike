// This file is part of TendaBike.
//
// TendaBike is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// TendaBike is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with TendaBike. If not, see <https://www.gnu.org/licenses/>.

//! Store seam integration suite (issue #406).
//!
//! Loads the standard prepopulated domain fixture into a real Postgres
//! database and verifies the store's I/O contract: the fixture roundtrip,
//! and the store-path rules that only the database adapter can exercise —
//! attachment delete identity, successor lookup, adjacent-merge trigger,
//! `find_part_attached_already` for a never-attached part, activity
//! duplicate-id rejection, update field preservation, time-range boundary,
//! user+time minute match, and `get_all` ordering.
//!
//! Domain-level rules (attach, detach, dispose, upsert, delete, and the
//! `ApiWrite`/`ApiRead` dispatch paths) are owned by the in-memory suite in
//! `tb_domain`; this suite asserts only what the store adapter itself must
//! get right.
//!
//! The suite is deterministic: all tests run serialized against one freshly
//! created scratch database; a one-time seed loads the fixture into it, and
//! every test opens a transaction with the sequences reset just past the
//! fixture ids and rolls back, so each test starts from exactly the
//! prepopulated state.
//!
//! The suite manages its scratch database at `SCRATCH_DATABASE_URL` (falling
//! back to `.env`): on first use it force-drops any database left behind by
//! a previous run, creates a fresh one via `MigrateDatabase`, runs the
//! migrations, and seeds the fixture; nothing is dropped at the end of the
//! run, so the next run starts from the same clean slate. The URL's user
//! must hold createdb rights on that server. The suite never connects to
//! `DATABASE_URL` or `DB_URL`, which point at the developer's working
//! database: the original `DATABASE_URL`-based design seeded that database
//! in place, and a local run destroyed real data. It reads the two variables
//! only to refuse a collision — a scratch URL that equals either fails the
//! run instead of dropping real data.
//!
//! The suite is ignored by default: every test carries
//! `#[ignore]`, so a plain `cargo test --workspace` run (no database)
//! reports the 11 tests ignored and executes none of them,
//! and the existing in-memory suites stay green. It runs with
//! `cargo test -- --include-ignored` (the `--` matters: `--include-ignored`
//! is a libtest flag, not a cargo flag), and it fails loudly whenever it
//! cannot run: without `SCRATCH_DATABASE_URL`, every test fails with the
//! no-URL message, and a scratch database that cannot be prepared fails
//! every test with the one root-cause string. A test that verified nothing
//! never passes.
//!
//! The CI job that runs this suite against a real Postgres service is the
//! required `rust` job in `.github/workflows/test.yml` (issue #411): it
//! carries a `postgres:18` service, sets `SCRATCH_DATABASE_URL` to a scratch
//! database, and runs `cargo test --release -- --include-ignored`, so the
//! suite is part of the one required gate.
//! `database_is_reachable` is the named canary: it fails the job loudly when
//! the scratch database cannot be prepared.

use std::time::Duration;

use tb_domain::test_support::{
    fixtures::{sample_purchase_date, test_session},
    part_type_ids,
};
use tb_domain::{
    ActTypeId, Activity, ActivityId, ActivityStore, Attachment, AttachmentStore, MAX_TIME, Part,
    PartId, PartStore, PartTypeId, SNAPSHOT_JSON, StoreSnapshot, UsageId, UsageStore, UserId,
    UserStore,
};
use time::{OffsetDateTime, macros::datetime};
use tokio::sync::MutexGuard;
use uuid::Uuid;

use part_type_ids::{BIKE, CHAIN};

#[path = "common/scratch.rs"]
mod scratch;
use scratch::*;

/// A test handle: one transaction on the shared test database. The lock is
/// held for the whole test so all tests in this suite run serialized; the
/// transaction is rolled back when the store is dropped.
struct Seam {
    /// Declared first so it is dropped first: the transaction rolls back
    /// while the lock below is still held.
    store: tb_sqlx::SqlxConn<'static>,
    /// Declared last so it is dropped last: the lock stays held while the
    /// test's transaction is rolled back on drop.
    _lock: MutexGuard<'static, ()>,
}

/// Open a fresh fixture transaction for a test, or the root-cause string of
/// why the seam cannot be opened (the test then fails loudly with it).
///
/// The lock first: the one-time scratch-database setup and the pool
/// connection (and its no-op migration run) are taken while the lock is
/// held, so tests never contend for the database.
///
/// The fixture builder (`build_workshop_store`) seeds the scratch database
/// from the `StoreSnapshot` fixture and returns a `Store` over that Postgres
/// connection. The pool is local to the builder: it is dropped when the
/// builder returns, and the transaction keeps the pool alive internally
/// through its own `Arc` handle until it is rolled back.
async fn seam() -> Result<Seam, String> {
    let lock = LOCK.lock().await;
    let store = build_workshop_store(None).await?;
    Ok(Seam { store, _lock: lock })
}

/// Run a test body against a fresh fixture transaction: opens the seam
/// (failing the test loudly with the root-cause string when it cannot — this
/// machine has no `SCRATCH_DATABASE_URL`, or the scratch database could not
/// be prepared), hands the store to the body by value, and returns the
/// body's result. The body never commits, so the transaction is rolled
/// back when the body's future drops; the `Seam`'s lock stays held until
/// that drop, so the suite still runs serialized — the destructure binds
/// `_lock` before `store`, and locals drop in reverse binding order, so the
/// lock outlives the store's rollback.
async fn with_seam<R>(f: impl FnOnce(tb_sqlx::SqlxConn<'static>) -> R) -> tb_domain::TbResult<()>
where
    R: std::future::Future<Output = tb_domain::TbResult<()>>,
{
    let Seam { _lock, store } = match seam().await {
        Ok(seam) => seam,
        Err(err) => panic!("{err}"),
    };
    f(store).await
}

// ---------------------------------------------------------------------------
// Sample data helpers, mirroring the in-memory suite's helpers
// ---------------------------------------------------------------------------

/// The in-memory suite's first attachment time (2024-01-01).
fn attachment_time() -> OffsetDateTime {
    datetime!(2024-01-01 00:00 UTC)
}

/// The in-memory suite's later attachment time (2024-06-01).
fn later_time() -> OffsetDateTime {
    datetime!(2024-06-01 00:00 UTC)
}

/// The in-memory suite's activity start (2023-11-14T22:13:20Z).
fn activity_start() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// An attachment with a fresh usage id, as `Attachment::new` builds it (that
/// constructor is `pub(crate)` in the domain, so the seam builds the same
/// value from the public fields).
fn att(
    part: PartId,
    attached: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
    detached: OffsetDateTime,
) -> Attachment {
    Attachment {
        part_id: part,
        attached,
        gear,
        hook,
        detached,
        usage: UsageId::from(Uuid::now_v7()),
    }
}

/// A test ride with the in-memory suite's sample metrics (one hour, 3500s,
/// 50km, 500m up, 300m down, 1000W), parameterized only by the fields the
/// call sites vary: the id, the name, the start instant, and the gear.
fn ride(id: i64, name: &str, start: OffsetDateTime, gear: Option<PartId>) -> Activity {
    Activity {
        id: ActivityId::new(id),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: name.to_string(),
        start,
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: Some(300),
        energy: Some(1000),
        gear,
        device_name: None,
        external_id: None,
    }
}

/// Create a part like the in-memory tests do.
async fn create_part(
    name: &str,
    vendor: &str,
    model: &str,
    what: PartTypeId,
    store: &mut tb_sqlx::SqlxConn<'_>,
) -> Part {
    Part::create(
        name.to_string(),
        vendor.to_string(),
        model.to_string(),
        what,
        None,
        sample_purchase_date(),
        &test_session(),
        store,
    )
    .await
    .expect("creating a part works")
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

/// The named canary. The `rust` CI job sets `SCRATCH_DATABASE_URL`
/// unconditionally, so whenever this test runs the scratch database must
/// actually be preparable (dropped if a previous run left it behind, created,
/// migrated, and seeded): the failure is reported here with the suite's
/// unified root-cause string. The bound makes a blackholed endpoint fail in
/// seconds instead of burning the pool's 30-second acquire timeout per test.
///
/// With no `SCRATCH_DATABASE_URL` — a local machine without a scratch
/// database — it fails with the no-URL message like the other tests; the
/// `#[ignore]` attribute keeps it out of plain runs.
#[tokio::test]
#[ignore]
async fn database_is_reachable() {
    let Some(url) = scratch_url() else {
        panic!("{NO_SCRATCH_URL}");
    };
    // Under the lock, like the other tests: the one-time setup runs exactly
    // once per run, and this test is where its failure is reported. The
    // guard's only use of the lock is to hold it for the setup, hence the
    // underscore.
    let _lock = LOCK.lock().await;
    let outcome =
        tokio::time::timeout(Duration::from_secs(10), SETUP.get_or_init(|| setup(&url))).await;
    match outcome {
        Ok(Setup::Ready(_)) => {}
        Ok(Setup::Failed(err)) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: {err}"
        ),
        Err(_) => panic!(
            "SCRATCH_DATABASE_URL is set ({url}) but the scratch database could \
             not be prepared: it did not become ready within 10s — is the \
             Postgres service running?"
        ),
    }
    // `_lock` is dropped here, with this test's runtime.
}

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// The standard prepopulated fixture, the same snapshot the in-memory suite
/// uses.
fn fixture() -> StoreSnapshot {
    serde_json::from_str(SNAPSHOT_JSON).expect("SNAPSHOT_JSON must be valid JSON")
}

/// The prepopulated fixture loads back from the database unchanged.
#[tokio::test]
#[ignore]
async fn fixture_roundtrip() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let snap = fixture();

        // users
        let user = UserStore::user_get(&mut store, UserId::from(1)).await?;
        assert_eq!(user.name, snap.users[0].name);
        assert_eq!(user.firstname, snap.users[0].firstname);
        assert_eq!(user.is_admin, snap.users[0].is_admin);
        assert_eq!(user.avatar, snap.users[0].avatar);
        assert_eq!(user.onboarding_status, snap.users[0].onboarding_status);

        // parts
        for part in &snap.parts {
            let stored = store.partid_get_part(part.id).await?;
            assert_eq!(&stored, part, "part {} roundtrip differs", part.id);
        }

        // attachments
        let mut counted = 0;
        for part in &snap.parts {
            let stored = store.attachments_all_by_part(part.id).await?;
            let expected = snap
                .attachments
                .iter()
                .filter(|a| a.part_id == part.id)
                .collect::<Vec<_>>();
            assert_eq!(
                stored.len(),
                expected.len(),
                "part {} attachment count",
                part.id
            );
            for expected in &expected {
                assert!(
                    stored.iter().any(|a| a == *expected),
                    "part {} is missing one of its attachments",
                    part.id
                );
            }
            counted += stored.len();
        }
        assert_eq!(counted, snap.attachments.len());

        // usages
        for usage in &snap.usages {
            let stored = UsageStore::usage_get(&mut store, usage.id).await?;
            assert_eq!(stored.as_ref(), Some(usage), "usage {} differs", usage.id);
        }

        // activities
        let acts = store.get_all(&UserId::from(1)).await?;
        assert_eq!(acts.len(), snap.activities.len());
        for expected in &snap.activities {
            assert!(
                acts.iter().any(|a| a == expected),
                "activity {} ({}) missing",
                expected.id,
                expected.name
            );
        }

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Attachment: store-path rules
// ---------------------------------------------------------------------------

/// An attachment delete addresses its row by part + attach time — the
/// database's key — so a stale gear field in the argument does not change
/// which row is deleted (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_delete_identity() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike1 = create_part("Bike 1", "TendaBike", "Standard", BIKE, &mut store).await;
        let bike2 = create_part("Bike 2", "TendaBike", "Pro", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        let t1 = attachment_time();
        let a1 = att(chain.id, t1, bike1.id, BIKE, MAX_TIME);
        AttachmentStore::attachment_create(&mut store, a1).await?;

        // A stale gear field in the argument — the row is still addressed by
        // part + attach time and is deleted.
        let stale = Attachment {
            gear: bike2.id,
            ..a1
        };
        let deleted = AttachmentStore::attachment_delete(&mut store, stale).await?;
        assert_eq!(deleted.gear, bike1.id);

        let atts = store.attachments_all_by_part(chain.id).await?;
        assert!(atts.is_empty(), "the row was deleted");

        Ok(())
    })
    .await
}

/// Finding the successor of an attachment: another part, of the same type,
/// at the same hook, attached later — the part's own later rows never count
/// (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_find_successor() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;
        let chain2 = create_part("Chain 2", "KMC", "X10", CHAIN, &mut store).await;

        // chain: t1..t2 and its own later row t2..MAX (never its successor);
        // chain2 takes the same hook at t2.
        store
            .attachment_create(att(
                chain.id,
                attachment_time(),
                bike.id,
                CHAIN,
                later_time(),
            ))
            .await?;
        store
            .attachment_create(att(chain.id, later_time(), bike.id, CHAIN, MAX_TIME))
            .await?;
        store
            .attachment_create(att(chain2.id, later_time(), bike.id, CHAIN, MAX_TIME))
            .await?;

        let successor = store
            .attachment_find_successor(chain.id, bike.id, CHAIN, attachment_time(), CHAIN)
            .await?
            .expect("the other chain's later row is the successor");
        assert_eq!(successor.part_id, chain2.id);
        assert_eq!(successor.attached, later_time());

        Ok(())
    })
    .await
}

/// Finding the attachment already attached to a part: the row of this part
/// at this gear and hook that ended exactly at the query time — the
/// adjacent-merge trigger (unified rule #407).
#[tokio::test]
#[ignore]
async fn attachment_find_part_attached_already() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        // Row t1..t2.
        store
            .attachment_create(att(
                chain.id,
                attachment_time(),
                bike.id,
                CHAIN,
                later_time(),
            ))
            .await?;

        // The row covers t1 but does not end there — not the merge trigger.
        let found = store
            .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, attachment_time())
            .await?;
        assert!(found.is_none());

        // The row ends exactly at t2 — the adjacent-merge trigger.
        let found = store
            .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, later_time())
            .await?
            .expect("the row ending exactly at the time is the merge trigger");
        assert_eq!(found.attached, attachment_time());

        Ok(())
    })
    .await
}

/// `find_part_attached_already` for a part that was never attached is
/// `None` on both stores (no divergence).
#[tokio::test]
#[ignore]
async fn find_attached_already_never_attached() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Main Bike", "TendaBike", "Standard", BIKE, &mut store).await;
        let chain = create_part("Test Chain", "Shimano", "CN-M510", CHAIN, &mut store).await;

        let found = store
            .attachment_find_part_attached_already(chain.id, bike.id, CHAIN, attachment_time())
            .await?;
        assert!(found.is_none());

        Ok(())
    })
    .await
}

// ---------------------------------------------------------------------------
// Activity: store-path rules
// ---------------------------------------------------------------------------

/// The activity id is a primary key (issue #405): the INSERT hits the key
/// on a duplicate id and maps to `Error::DatabaseFailure`.
#[tokio::test]
#[ignore]
async fn activity_create_rejects_duplicate_id() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let ride = ride(100, "Ride", activity_start(), None);
        store.activity_create(ride.clone()).await?;

        // The first ride is stored, and a different id still succeeds.
        let stored = store
            .activity_read_by_id(ActivityId::new(100))
            .await?
            .expect("the first ride is stored");
        assert_eq!(stored.id, ActivityId::new(100));
        let mut other = ride.clone();
        other.id = ActivityId::new(101);
        store.activity_create(other).await?;

        // The duplicate create comes last: a failed statement aborts the
        // Postgres transaction, so nothing may follow it inside this test.
        let err = store
            .activity_create(ride.clone())
            .await
            .expect_err("a duplicate activity id must fail");
        assert!(
            matches!(err, tb_domain::Error::DatabaseFailure(_)),
            "a duplicate activity id must be a DatabaseFailure, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// Activity update replaces the data fields but keeps the stored row's
/// `utc_offset`, `device_name`, and `external_id` — the fields a frontend
/// round-trip can lose; one rule on both stores (#408 rule 1).
#[tokio::test]
#[ignore]
async fn activity_update_preserves_fields() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        // A ride stored at 22:13:20+01:00, with device metadata.
        let act = Activity {
            device_name: Some("Garmin Edge".to_string()),
            external_id: Some("garmin_12345".to_string()),
            ..ride(
                100,
                "Morning Ride",
                activity_start().to_offset(time::UtcOffset::from_whole_seconds(3600).unwrap()),
                None,
            )
        };
        store.activity_create(act.clone()).await?;

        // A client that lost the device data updates the name only, with the
        // start in a UTC representation.
        let lossy = Activity {
            name: "Modified Ride".to_string(),
            start: activity_start().to_offset(time::UtcOffset::UTC),
            device_name: None,
            external_id: None,
            ..act.clone()
        };
        let updated = store.activity_update(lossy).await?;

        // The rule: the data fields are replaced …
        assert_eq!(updated.name, "Modified Ride");
        assert_eq!(
            updated.start.unix_timestamp(),
            activity_start().unix_timestamp()
        );
        // … and the stored row keeps its offset, device name, and external id.
        assert_eq!(updated.start.offset().whole_seconds(), 3600);
        assert_eq!(updated.device_name.as_deref(), Some("Garmin Edge"));
        assert_eq!(updated.external_id.as_deref(), Some("garmin_12345"));

        // An update of a missing activity is a NotFound on both stores.
        let ghost = Activity {
            id: ActivityId::new(999),
            ..act
        };
        let err = store.activity_update(ghost).await;
        assert!(
            matches!(err, Err(tb_domain::Error::NotFound(_))),
            "updating a missing activity must be NotFound, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// Listing activities in a time range: `begin` is included, `end` is
/// excluded — one rule on both stores (#408 rule 2).
#[tokio::test]
#[ignore]
async fn activity_find_range_boundary() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = create_part("Road Bike", "Trek", "Domane", BIKE, &mut store).await;
        let start = datetime!(2024-02-01 12:00 UTC);
        let act = ride(100, "Boundary Ride", start, Some(bike.id));
        store.activity_create(act).await?;

        // A window that ends exactly at the activity's start: the boundary ride
        // is outside the range, because end is exclusive.
        let begin = start - time::Duration::minutes(30);
        let found = store
            .activities_find_by_gear_and_time(bike.id, begin, start)
            .await?;
        assert!(
            found.is_empty(),
            "a ride exactly at the range end must be excluded, got {found:?}"
        );

        // A window that begins exactly at the activity's start: begin is
        // inclusive.
        let found = store
            .activities_find_by_gear_and_time(bike.id, start, start + time::Duration::minutes(30))
            .await?;
        assert_eq!(
            found.len(),
            1,
            "a ride exactly at the range begin must be included"
        );
        assert_eq!(found[0].id, ActivityId::new(100));

        Ok(())
    })
    .await
}

/// The import lookup by user and time matches by the minute, not the
/// instant: the activity's local wall-clock minute (its start in the stored
/// offset) must equal the query's UTC wall-clock minute; one rule on both
/// stores (#408 rule 3). Zero matches is NotFound, exactly one returns the
/// activity, and the maintainer-confirmed duplicate rule applies: two or
/// more activities in the same minute is an Error::Ambiguous.
#[tokio::test]
#[ignore]
async fn activity_get_by_user_and_time() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let start = datetime!(2024-02-01 12:00:30 UTC);
        let act = ride(100, "Minute Ride", start, None);
        store.activity_create(act.clone()).await?;

        // A different instant of the same minute: the minute match finds the
        // activity (a zero-offset ride's local minute is its UTC minute).
        let query = start + time::Duration::seconds(15);
        let found = store.get_by_user_and_time(UserId::from(1), query).await?;
        assert_eq!(found.id, ActivityId::new(100));

        // A different minute: no match.
        let other_minute = start + time::Duration::minutes(2);
        let err = store
            .get_by_user_and_time(UserId::from(1), other_minute)
            .await;
        assert!(
            matches!(err, Err(tb_domain::Error::NotFound(_))),
            "a different minute must not match, got {err:?}"
        );

        // A ride with a +02:00 offset matches only queries whose UTC wall clock
        // equals the ride's local minute (10:53), not its UTC minute (08:53).
        let local_start = OffsetDateTime::from_unix_timestamp(1706777630) // 08:53:50 UTC
            .unwrap()
            .to_offset(time::UtcOffset::from_whole_seconds(7200).unwrap());
        let act2 = Activity {
            id: ActivityId::new(101),
            start: local_start,
            ..act
        };
        store.activity_create(act2.clone()).await?;

        let query = OffsetDateTime::from_unix_timestamp(1706777630 + 7200 - 20).unwrap(); // 10:53:30 UTC — the ride's local minute
        let found = store.get_by_user_and_time(UserId::from(1), query).await?;
        assert_eq!(found.id, ActivityId::new(101));

        let next_minute = query + time::Duration::seconds(60); // 10:54:30 UTC
        let err = store
            .get_by_user_and_time(UserId::from(1), next_minute)
            .await;
        assert!(
            matches!(err, Err(tb_domain::Error::NotFound(_))),
            "a different minute must not match, got {err:?}"
        );

        // The confirmed duplicate rule: a second ride in the first ride's minute
        // makes the lookup ambiguous — an error, not a silent first-match, so a
        // conflicting CSV row lands in the bad list.
        let dup = Activity {
            id: ActivityId::new(102),
            start: start + time::Duration::seconds(15), // 12:00:45 — the first ride's minute
            ..act2.clone()
        };
        store.activity_create(dup).await?;
        let err = store
            .get_by_user_and_time(UserId::from(1), start + time::Duration::seconds(20))
            .await;
        assert!(
            matches!(err, Err(tb_domain::Error::Ambiguous(_))),
            "two same-minute rides must return Error::Ambiguous, got {err:?}"
        );

        Ok(())
    })
    .await
}

/// `get_all` returns the user's activities in ascending start instant —
/// the database's `ORDER BY start` is the one rule on both stores (issue
/// #405). The later ride is created first; the listing must not follow
/// creation order.
#[tokio::test]
#[ignore]
async fn activity_get_all_orders_by_start() -> tb_domain::TbResult<()> {
    with_seam(|mut store| async move {
        let bike = PartId::from(1);

        // The later ride is created first.
        let later = ride(
            100,
            "Later Ride",
            activity_start() + time::Duration::hours(1),
            Some(bike),
        );
        store.activity_create(later).await?;

        // The earlier ride is created second.
        let earlier = ride(101, "Earlier Ride", activity_start(), Some(bike));
        store.activity_create(earlier).await?;

        // The three fixture rides (May 2023) precede the two created rides
        // (November 2023), and the created rides come back in start order,
        // not creation order.
        let acts = store.get_all(&UserId::from(1)).await?;
        let names: Vec<&str> = acts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Morning Ride",
                "Hill Repeats",
                "Recovery Spin",
                "Earlier Ride",
                "Later Ride"
            ]
        );

        Ok(())
    })
    .await
}
