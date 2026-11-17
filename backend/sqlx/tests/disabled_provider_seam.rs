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

//! The disabled-provider seam suite (ADR-0016, issue #908): the graceful
//! freeze of a provider whose client id is unset, driven on the live
//! adapter (the real `Registry`, the real `UnifiedLane`, the real
//! dispatch) against real Postgres.
//!
//! The suite's one-time setup initializes the provider config with Strava
//! **disabled** (its client id unset) and intervals.icu enabled — the
//! opposite of every other suite's all-enabled config, which is why this
//! is its own binary (the provider config is process-global) on its own
//! suffixed scratch database (`<scratch>_disab`).
//!
//! What the seam tests verify, end to end on the live adapter:
//!
//! - the **enabled set** is derived from the config: Strava's client id
//!   unset yields `[Icu]` (the enum's definition order);
//! - a **login** on the disabled provider fails with a clear
//!   `Unavailable` "disabled on this server" error (not a 500-class DB
//!   failure, not a token exchange);
//! - the disabled provider's **webhook** is inert: the intake answers
//!   `Ok` and queues nothing;
//! - a **sync** of a disabled provider's user fails with the clear
//!   `Unavailable` error (the graceful freeze is loud, not silent);
//! - a disabled provider's user **keeps reading**: the `user_provider`
//!   identity read still resolves;
//! - a **queued event** of a disabled provider stays queued (the lane's
//!   `process` declines without dropping it): re-adding the credential
//!   restores the provider and its syncs;
//! - the **enabled provider keeps working**: its webhook queues its rows.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::*;

use std::time::Duration;

use tokio::sync::MutexGuard;

use tb_domain::test_support::mock_session_admin;
use tb_domain::{Error, UserId};
use tb_exec::providerservice::ProviderService;
use tb_exec::{DetachedBegin, DomainFacadeImpl, Registry};
use tb_provider::strava::StravaId;
use tb_provider::{EntityFacadeImpl, Provider, ProviderLane, UnifiedLane};
use tb_sqlx::{DbPool, SqlxConn};

/// The scratch database suffix this suite carries (see the module docs).
const DB_SUFFIX: &str = "_disab";

/// The bound for one direct op in a test: long enough for a slow scratch
/// database, short enough that a wedged op fails the test instead of
/// hanging the suite.
const TEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Initialize the provider config once for the suite's process: Strava
/// **disabled** (its client id unset — the graceful freeze under test,
/// ADR-0016), intervals.icu enabled with a known webhook secret.
fn init_provider_config_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tb_provider::init(tb_provider::ProviderConfig {
            strava_client_id: None,
            strava_client_secret: None,
            strava_redirect_url: "http://localhost:8000/strava/token".to_string(),
            icu_client_id: Some("seam".to_string()),
            icu_client_secret: Some("seam".to_string()),
            icu_redirect_url: "http://localhost:8000/icu/token".to_string(),
            icu_webhook_secret: "seam-webhook-secret".to_string(),
        });
    });
}

/// The in-memory suite's session identity: the fixture's one user and its
/// Strava id (the same pair the other seam suites use).
fn test_identity() -> (UserId, StravaId) {
    (UserId::from(1), StravaId::from(42))
}

/// One test's hold on the scratch database: a pool for the test's runtime
/// plus the run's lock (declared last so it drops last).
struct DisabSeam {
    pool: DbPool,
    _lock: MutexGuard<'static, ()>,
}

/// Open one test's seam: take the run's lock, prepare the suffixed scratch
/// database, build this test's pool, and reset it to the pristine fixture.
async fn ready() -> DisabSeam {
    let _lock = LOCK.lock().await;
    let url = match suffixed_scratch_url(DB_SUFFIX) {
        Some(url) => url,
        None => panic!("{NO_SCRATCH_URL}"),
    };
    build_workshop_store(Some(DB_SUFFIX))
        .await
        .expect("the disabled-provider scratch database could not be prepared");
    let pool = pool(&url)
        .await
        .expect("the scratch database pool could not be built");
    fresh_fixture(&pool)
        .await
        .expect("re-seeding the fixture failed");
    DisabSeam { pool, _lock }
}

/// One test's registry over the live pool (the direct ops' shape).
fn registry(
    pool: &DbPool,
) -> Registry<DetachedBegin<DbPool>, UnifiedLane<SqlxConn<'static>>, DomainFacadeImpl> {
    Registry::new(
        DetachedBegin::new(pool.clone()),
        DomainFacadeImpl,
        UnifiedLane::new(EntityFacadeImpl),
        Duration::from_millis(300),
    )
}

/// Seed the fixture user's Strava row (and, when `queue_event`, one queued
/// event for it) — the direct ops' setup shape.
async fn seed_strava_user(pool: &DbPool, queue_event: bool) {
    let (user, strava) = test_identity();
    let mut conn = pool.begin().await.expect("a transaction");
    sqlx::query(
        "INSERT INTO strava_users (id, tendabike_id, refresh_token) VALUES ($1, $2, 'tok')",
    )
    .bind(i32::from(strava))
    .bind(i32::from(user))
    .execute(&mut **conn)
    .await
    .expect("seeding the strava user");
    if queue_event {
        sqlx::query(
            "INSERT INTO strava_events \
             (object_type, object_id, aspect_type, updates, owner_id, subscription_id, event_time) \
             VALUES ('activity', 7, 'create', '{}', $1, 0, 100)",
        )
        .bind(i32::from(strava))
        .execute(&mut **conn)
        .await
        .expect("seeding the queued event");
    }
    conn.commit().await.expect("committing the seed");
}

/// Seed the fixture user's intervals.icu row (the enabled provider's
/// identity for the webhook test).
async fn seed_icu_user(pool: &DbPool) {
    let (user, _) = test_identity();
    let mut conn = pool.begin().await.expect("a transaction");
    sqlx::query(
        "INSERT INTO icu_users (tbid, icu_athlete_id, access_token, disabled) \
         VALUES ($1, 'icu-7', 'tok', false)",
    )
    .bind(i32::from(user))
    .execute(&mut **conn)
    .await
    .expect("seeding the icu user");
    conn.commit().await.expect("committing the seed");
}

/// The Strava queue's row count for `strava` on a fresh connection.
async fn strava_queue_count(pool: &DbPool, strava: StravaId) -> i64 {
    let mut conn = pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM strava_events WHERE owner_id = $1")
        .bind(i32::from(strava))
        .fetch_one(&mut **conn)
        .await
        .expect("reading the queue");
    drop(conn);
    n
}

/// A raw Strava webhook body: an activity-create event for the fixture
/// user's Strava id.
fn strava_activity_body() -> String {
    let (_, strava) = test_identity();
    format!(
        r#"{{"object_type":"activity","object_id":11,"aspect_type":"create","updates":{{}},"owner_id":{},"subscription_id":1,"event_time":1767348000}}"#,
        i32::from(strava)
    )
}

/// A raw intervals.icu webhook body: one `ACTIVITY_UPLOADED` event for the
/// seeded icu athlete, under the suite's shared secret.
fn icu_activity_body() -> String {
    r#"{"secret":"seam-webhook-secret","events":[{"athlete_id":"icu-7","type":"ACTIVITY_UPLOADED","timestamp":"2026-01-01T00:00:00Z","activity":{"id":"icu-act-1"}}]}"#
        .to_string()
}

// ---------------------------------------------------------------------------
// Availability
// ---------------------------------------------------------------------------

/// The named canary, mirroring the other seam suites': whenever the suite
/// runs, this suite's scratch database must actually be preparable.
#[tokio::test]
#[ignore]
async fn database_is_reachable() {
    canary(suffixed_scratch_url(DB_SUFFIX)).await;
}

// ---------------------------------------------------------------------------
// The enabled set
// ---------------------------------------------------------------------------

/// The enabled set is derived from the config (ADR-0016): Strava's client
/// id is unset, so only intervals.icu is enabled — in the `Provider`
/// enum's definition order. No database involved: the set is a pure
/// config read.
#[test]
fn enabled_set_derives_from_the_config() {
    init_provider_config_once();
    assert_eq!(tb_provider::enabled_providers(), &[Provider::Icu]);
}

// ---------------------------------------------------------------------------
// The graceful freeze
// ---------------------------------------------------------------------------

/// A login on the disabled provider fails with the clear `Unavailable`
/// "disabled on this server" error (ADR-0016): not a 500-class DB failure,
/// not a token exchange against missing credentials.
#[tokio::test]
#[ignore]
async fn login_of_a_disabled_provider_is_unavailable() {
    let seam = ready().await;
    init_provider_config_once();
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        reg.login(
            Provider::Strava,
            "seam-code",
            "read,activity:read_all,profile:read_all",
        ),
    )
    .await
    .expect("the login must finish in time");
    match outcome {
        Err(Error::Unavailable(msg)) => {
            assert!(
                msg.contains("strava") && msg.contains("disabled on this server"),
                "the message must name the provider and state the freeze: {msg}"
            );
        }
        other => panic!("a disabled provider's login must be a clear Unavailable, got: {other:?}"),
    }
}

/// The disabled provider's webhook is inert (ADR-0016): the intake answers
/// `Ok` (the route answers 200, not a 500) and queues nothing.
#[tokio::test]
#[ignore]
async fn webhook_of_a_disabled_provider_is_inert() {
    let seam = ready().await;
    init_provider_config_once();
    seed_strava_user(&seam.pool, false).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        reg.accept_event(Provider::Strava, &strava_activity_body()),
    )
    .await
    .expect("the intake must finish in time");
    outcome.expect("a disabled provider's webhook must answer Ok, not fail");

    let (_, strava) = test_identity();
    assert_eq!(
        strava_queue_count(&seam.pool, strava).await,
        0,
        "the inert webhook must queue nothing"
    );
}

/// A sync of a disabled provider's user fails with the clear `Unavailable`
/// error (ADR-0016): the graceful freeze is loud, not a silent no-op.
#[tokio::test]
#[ignore]
async fn sync_of_a_disabled_provider_is_unavailable() {
    let seam = ready().await;
    init_provider_config_once();
    let (user, _) = test_identity();
    seed_strava_user(&seam.pool, false).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        reg.sync(Box::new(mock_session_admin(user)), user, 100),
    )
    .await
    .expect("the sync must finish in time");
    match outcome {
        Err(Error::Unavailable(msg)) => {
            assert!(
                msg.contains("strava") && msg.contains("disabled on this server"),
                "the message must name the provider and state the freeze: {msg}"
            );
        }
        other => panic!("a disabled provider's sync must be a clear Unavailable, got: {other:?}"),
    }
}

/// A disabled provider's user keeps reading their data (ADR-0016): the
/// `user_provider` identity read still resolves — the freeze hides no data.
#[tokio::test]
#[ignore]
async fn reads_of_a_disabled_provider_still_work() {
    let seam = ready().await;
    init_provider_config_once();
    let (user, _) = test_identity();
    seed_strava_user(&seam.pool, false).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(TEST_TIMEOUT, reg.user_provider(user))
        .await
        .expect("the identity read must finish in time");
    assert_eq!(
        outcome.expect("the identity read must succeed"),
        Provider::Strava,
        "a disabled provider's user must keep their identity"
    );
}

/// A queued event of a disabled provider stays queued (ADR-0016): the
/// lane's `process` declines without dropping it — re-adding the
/// credential restores the provider and its syncs.
#[tokio::test]
#[ignore]
async fn queued_events_of_a_disabled_provider_stay_queued() {
    let seam = ready().await;
    init_provider_config_once();
    let (user, strava) = test_identity();
    seed_strava_user(&seam.pool, true).await;
    assert_eq!(strava_queue_count(&seam.pool, strava).await, 1);

    let lane = UnifiedLane::new(EntityFacadeImpl);
    let mut conn = seam.pool.begin().await.expect("a transaction");
    let outcome = tokio::time::timeout(TEST_TIMEOUT, lane.process(user, &mut conn))
        .await
        .expect("the process must finish in time");
    assert!(
        outcome.is_none(),
        "a disabled provider's queued event must be declined, not processed"
    );
    drop(conn);

    assert_eq!(
        strava_queue_count(&seam.pool, strava).await,
        1,
        "the declined event must stay queued (the freeze is reversible)"
    );
}

/// The enabled provider keeps working (ADR-0016): the freeze is
/// per-provider — intervals.icu's webhook verifies its secret and queues
/// its rows.
#[tokio::test]
#[ignore]
async fn the_enabled_provider_keeps_working() {
    let seam = ready().await;
    init_provider_config_once();
    seed_icu_user(&seam.pool).await;
    let reg = registry(&seam.pool);

    let outcome = tokio::time::timeout(
        TEST_TIMEOUT,
        reg.accept_event(Provider::Icu, &icu_activity_body()),
    )
    .await
    .expect("the intake must finish in time");
    outcome.expect("the enabled provider's webhook must succeed");

    let mut conn = seam.pool.begin().await.expect("a fresh connection");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM icu_events")
        .fetch_one(&mut **conn)
        .await
        .expect("reading the icu queue");
    drop(conn);
    assert_eq!(n, 1, "the enabled provider's webhook must queue its row");
}
