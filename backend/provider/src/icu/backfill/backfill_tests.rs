use std::sync::{Arc, Mutex};

use super::super::test_support::{icu_user, queue, queue_error, requests, setup};
use super::*;
use crate::facade::{Error, Summary};

type MockIcuDataStore = super::super::traits::MockIcuDataStore;

/// A minimal activity in a backfill batch response (the fields the domain
/// Activity records).
fn activity_json(id: &str, start: &str) -> String {
    format!(
        r#"{{"id":"{id}","type":"Ride","name":"Test Ride","start_date":"{start}","distance":10000,"moving_time":3600,"elapsed_time":3700,"total_elevation_gain":100.0,"total_elevation_loss":100.0,"gear":{{"id":"g1"}},"icu_joules":50000.0}}"#
    )
}

/// The user's icu athlete id fixture.
fn user_fixture(store: &mut MockIcuDataStore) {
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
}

/// The backfill URI for `oldest=0` (the onboarding case), with today's date
/// as the `newest` bound.
fn backfill_uri() -> String {
    let today = date_str(OffsetDateTime::now_utc().unix_timestamp());
    format!("/athlete/a1/activities?oldest=1970-01-01&newest={today}&limit=100&fields={FIELDS}")
}

// ─── the backfill walk ──────────────────────────────────────────────────────

#[tokio::test]
async fn backfill_imports_activities_directly() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    user_fixture(&mut store);
    // One batch of activities (fewer than LIMIT → end of history).
    queue(
        &backfill_uri(),
        &format!("[{}]", activity_json("10", "2023-06-01T10:00:00Z")),
    );
    let upserted: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let upserted_rec = upserted.clone();
    facade
        .expect_upsert_activity()
        .times(1)
        .returning(move |activity, _session, _store| {
            upserted_rec.lock().unwrap().push(activity.id.to_string());
            Ok(Summary::default())
        });
    // The gear ref resolution (part_by_source).
    facade
        .expect_part_by_source()
        .returning(|_source, _store| Ok(None));
    backfill(UserId::from(1), 0, &facade, &mut store)
        .await
        .unwrap();
    assert_eq!(
        upserted.lock().unwrap().as_slice(),
        ["icu:10"],
        "the activity is imported with the icu: prefix"
    );
}

#[tokio::test]
async fn backfill_stops_when_no_more_activities() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    user_fixture(&mut store);
    // Empty response → end of history.
    queue(&backfill_uri(), "[]");
    facade.expect_upsert_activity().times(0);
    backfill(UserId::from(1), 0, &facade, &mut store)
        .await
        .unwrap();
    assert!(
        requests().len() == 1,
        "only one batch is fetched (the empty one)"
    );
}

#[tokio::test]
async fn backfill_stops_before_fetching_when_rate_limited() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_limited();
    user_fixture(&mut store);
    // The proactive gate fires before any fetch.
    facade.expect_upsert_activity().times(0);
    backfill(UserId::from(1), 0, &facade, &mut store)
        .await
        .unwrap();
    assert!(
        requests().is_empty(),
        "no batch is fetched while the rate-limit budget is exhausted"
    );
}

#[tokio::test]
async fn backfill_stops_on_429() {
    let (mut store, mut facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    user_fixture(&mut store);
    queue_error(&backfill_uri(), Error::TryAgain("rate limited"));
    facade.expect_upsert_activity().times(0);
    backfill(UserId::from(1), 0, &facade, &mut store)
        .await
        .unwrap();
    assert_eq!(requests().len(), 1, "the walk stops after the 429");
}
