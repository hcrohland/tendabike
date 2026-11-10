use std::sync::{Arc, Mutex};

use super::super::test_support::{icu_user, queue, queue_error, requests, setup};
use super::*;
use tb_domain::Error;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

type MockIcuDataStore = super::super::traits::MockIcuDataStore;

/// A minimal activity in a backfill window response (the fields the queue row
/// needs: the id and the start time).
fn activity_json(id: &str, start: &str) -> String {
    format!(r#"{{"id":"{id}","start_date":"{start}"}}"#)
}

/// The user's icu athlete id fixture (the `icu_events` queue's key).
fn user_fixture(store: &mut MockIcuDataStore) {
    store
        .expect_icuser_get_by_tbid()
        .times(..)
        .returning(|_id| Ok(icu_user(UserId::from(1), "a1", true)));
}

// ─── the date-window walk ───────────────────────────────────────────────────

#[tokio::test]
async fn backfill_walks_all_windows_and_queues_activities() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    let now = 1_700_000_000i64;
    user_fixture(&mut store);
    // No queued events yet: the cursor starts at `now - 6 months`.
    store
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(None));
    // Six one-month windows, each with one activity.
    queue(
        "/athlete/a1/activities?oldest=2023-05-18&newest=2023-06-17",
        &format!("[{}]", activity_json("10", "2023-05-18T10:00:00Z")),
    );
    queue(
        "/athlete/a1/activities?oldest=2023-06-17&newest=2023-07-17",
        &format!("[{}]", activity_json("11", "2023-06-18T10:00:00Z")),
    );
    queue(
        "/athlete/a1/activities?oldest=2023-07-17&newest=2023-08-16",
        &format!("[{}]", activity_json("12", "2023-07-18T10:00:00Z")),
    );
    queue(
        "/athlete/a1/activities?oldest=2023-08-16&newest=2023-09-15",
        &format!("[{}]", activity_json("13", "2023-08-18T10:00:00Z")),
    );
    queue(
        "/athlete/a1/activities?oldest=2023-09-15&newest=2023-10-15",
        &format!("[{}]", activity_json("14", "2023-09-18T10:00:00Z")),
    );
    queue(
        "/athlete/a1/activities?oldest=2023-10-15&newest=2023-11-14",
        &format!("[{}]", activity_json("15", "2023-10-18T10:00:00Z")),
    );
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_rec = created.clone();
    store.expect_icu_event_create().returning(move |e| {
        created_rec.lock().unwrap().push(e);
        Ok(())
    });
    backfill(UserId::from(1), now, &mut store).await.unwrap();
    let created = created.lock().unwrap();
    assert_eq!(created.len(), 6, "one event per window's activity");
    assert!(
        created.iter().all(|e| e.object_type == "ACTIVITY_ANALYZED"),
        "the backfill events carry the ACTIVITY_ANALYZED object type"
    );
    // The event_time is the activity's start time (the resume cursor).
    assert_eq!(
        created.iter().map(|e| e.event_time).min(),
        Some(
            OffsetDateTime::parse("2023-05-18T10:00:00Z", &Rfc3339)
                .unwrap()
                .unix_timestamp()
        )
    );
}

#[tokio::test]
async fn backfill_resumes_from_newest_queued_event() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    let now = 1_700_000_000i64;
    user_fixture(&mut store);
    // A prior backfill queued events up to 1697408000: the walk resumes there.
    store
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(Some(1697408000)));
    // Only the final window is fetched (from the cursor to `now`).
    queue(
        "/athlete/a1/activities?oldest=2023-10-15&newest=2023-11-14",
        &format!("[{}]", activity_json("15", "2023-10-18T10:00:00Z")),
    );
    store.expect_icu_event_create().returning(|_e| Ok(()));
    backfill(UserId::from(1), now, &mut store).await.unwrap();
    assert_eq!(
        requests(),
        vec!["/athlete/a1/activities?oldest=2023-10-15&newest=2023-11-14".to_string()],
        "only the window after the cursor is fetched"
    );
}

#[tokio::test]
async fn backfill_stops_before_fetching_when_rate_limited() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_limited();
    let now = 1_700_000_000i64;
    user_fixture(&mut store);
    store
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(None));
    // The proactive gate fires before any fetch: the walk stops, no window is
    // fetched (the queued events, if any, commit).
    backfill(UserId::from(1), now, &mut store).await.unwrap();
    assert!(
        requests().is_empty(),
        "no window is fetched while the rate-limit budget is exhausted"
    );
}

#[tokio::test]
async fn backfill_stops_on_429_and_keeps_queued_events() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    let now = 1_700_000_000i64;
    user_fixture(&mut store);
    store
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(None));
    // The first window's fetch is rate-limited: the walk stops (Ok — the
    // events queued so far commit), and no further window is fetched.
    queue_error(
        "/athlete/a1/activities?oldest=2023-05-18&newest=2023-06-17",
        Error::TryAgain("rate limited"),
    );
    backfill(UserId::from(1), now, &mut store).await.unwrap();
    assert_eq!(
        requests().len(),
        1,
        "the walk stops after the 429 (no further window is fetched)"
    );
}

#[tokio::test]
async fn backfill_queues_nothing_when_history_is_empty() {
    let (mut store, _facade) = setup();
    super::super::ratelimit::test_set_not_limited();
    let now = 1_700_000_000i64;
    user_fixture(&mut store);
    store
        .expect_icu_events_get_latest_time()
        .returning(|_id| Ok(None));
    // Every window returns an empty list: nothing is queued.
    for (oldest, newest) in [
        ("2023-05-18", "2023-06-17"),
        ("2023-06-17", "2023-07-17"),
        ("2023-07-17", "2023-08-16"),
        ("2023-08-16", "2023-09-15"),
        ("2023-09-15", "2023-10-15"),
        ("2023-10-15", "2023-11-14"),
    ] {
        queue(
            &format!("/athlete/a1/activities?oldest={oldest}&newest={newest}"),
            "[]",
        );
    }
    store.expect_icu_event_create().times(0);
    backfill(UserId::from(1), now, &mut store).await.unwrap();
}
