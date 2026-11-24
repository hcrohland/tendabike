use super::{MockEventQueue, QueuedEvent, fail_event, get_event};
use crate::facade::{Error, Summary};
use crate::lane::QueueRead;

/// A queued event row for the queue-op tests: the identity the ops key on.
#[derive(Debug, Clone, PartialEq)]
struct TestEvent {
    id: Option<i32>,
    owner: i32,
    key: i64,
}

impl QueuedEvent for TestEvent {
    type Owner = i32;
    type Key = i64;
    fn id(&self) -> Option<i32> {
        self.id
    }
    fn owner(&self) -> &i32 {
        &self.owner
    }
    fn key(&self) -> &i64 {
        &self.key
    }
}

fn event(id: i32, owner: i32, key: i64) -> TestEvent {
    TestEvent {
        id: Some(id),
        owner,
        key,
    }
}

type MockQueue = MockEventQueue<TestEvent>;

// ─── get_event (the read-side rules) ────────────────────────────────────────

/// The rate-limit gate fires before any store read: budget exhausted →
/// `Limited` (the event stays queued).
#[tokio::test]
async fn get_event_limited_when_rate_limited() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| Some(123));
    // The gate fires first: no queue read.
    queue.expect_get_next_for_user().times(0);
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(matches!(read, QueueRead::Limited { until: 123 }));
}

/// An empty queue: no event for the owner → `Empty`.
#[tokio::test]
async fn get_event_empty_when_no_events() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| None);
    queue.expect_get_next_for_user().returning(|_user| Ok(None));
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(matches!(read, QueueRead::Empty));
}

/// A single queued event: the dedup no-ops (the event is the only row for its
/// key) and the event is returned.
#[tokio::test]
async fn get_event_returns_queued_event() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| None);
    queue
        .expect_get_next_for_user()
        .returning(|_user| Ok(Some(event(1, 1, 10))));
    // The event is the only queued row for its key: the dedup no-ops.
    queue
        .expect_get_later()
        .returning(|_key, _owner| Ok(vec![event(1, 1, 10)]));
    queue.expect_delete_batch().times(0);
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(matches!(read, QueueRead::Event(_)));
}

/// The queue's read-side dedup (spec #996): for a burst of events on one key,
/// only the latest row is returned — the older duplicates are batch-deleted
/// (one API call per object, not per event).
#[tokio::test]
async fn get_event_returns_latest_and_drops_older() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| None);
    // Two events for the same key: the read returns the latest (id 2) and
    // drops the older (id 1).
    queue
        .expect_get_next_for_user()
        .returning(|_user| Ok(Some(event(1, 1, 10))));
    queue
        .expect_get_later()
        .returning(|_key, _owner| Ok(vec![event(1, 1, 10), event(2, 1, 10)]));
    let dropped: std::sync::Arc<std::sync::Mutex<Vec<Option<i32>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = dropped.clone();
    queue.expect_delete_batch().returning(move |values| {
        rec.lock().unwrap().extend(values);
        Ok(())
    });
    let read = get_event(1, &mut queue).await.unwrap();
    match read {
        QueueRead::Event(e) => {
            assert_eq!(e.id, Some(2), "the latest event is returned")
        }
        other => panic!("the latest event is returned, got {other:?}"),
    }
    assert_eq!(
        dropped.lock().unwrap().as_slice(),
        [Some(1)],
        "the older event is dropped"
    );
}

/// The dedup is keyed on the row's own `(key, owner)`: the `get_later` query
/// carries the event's key and owner (a `SYNC`-style sentinel key groups with
/// nothing else, so the dedup no-ops for it — no batch delete fires).
#[tokio::test]
async fn get_event_dedup_is_keyed_on_the_row() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| None);
    // A sentinel-keyed row (the icu `SYNC` row's `"sync"` key, generalized):
    // the dedup query is keyed on the row's own key and owner.
    queue
        .expect_get_next_for_user()
        .returning(|_user| Ok(Some(event(7, 1, 999))));
    queue.expect_get_later().returning(|key, owner| {
        assert_eq!(key, 999, "keyed on the row's own key");
        assert_eq!(owner, 1, "keyed on the row's own owner");
        Ok(vec![event(7, 1, 999)])
    });
    queue.expect_delete_batch().times(0);
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(
        matches!(read, QueueRead::Event(e) if e.id == Some(7)),
        "the row is returned as-is"
    );
}

// ─── fail_event (the four-arm policy) ───────────────────────────────────────

/// A rate limit (`TryAgain`) keeps the event queued: no queue-side effect, an
/// empty summary (the next `get_event` probe reports `Limited`).
#[tokio::test]
async fn fail_event_keep_queued_is_a_noop() {
    let mut queue = MockQueue::new();
    queue
        .expect_provider_id()
        .returning(|| crate::Provider::Icu);
    queue.expect_disable_user().times(0);
    queue.expect_delete_event().times(0);
    let res = fail_event(&event(1, 1, 10), Error::TryAgain("rate limit"), &mut queue)
        .await
        .unwrap();
    assert_eq!(res, Summary::default());
}

/// Dead auth (`NotAuth`) disables the user (which drops their queued events):
/// the disable commits with the caller's transaction.
#[tokio::test]
async fn fail_event_disable_user_disables() {
    let mut queue = MockQueue::new();
    queue
        .expect_provider_id()
        .returning(|| crate::Provider::Icu);
    let disabled: std::sync::Arc<std::sync::Mutex<Option<i32>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let rec = disabled.clone();
    queue.expect_disable_user().returning(move |user| {
        rec.lock().unwrap().replace(user);
        Ok(())
    });
    queue.expect_delete_event().times(0);
    let res = fail_event(
        &event(1, 42, 10),
        Error::NotAuth("token dead".into()),
        &mut queue,
    )
    .await
    .unwrap();
    assert_eq!(res, Summary::default());
    assert_eq!(
        disabled.lock().unwrap().as_ref(),
        Some(&42),
        "the event's owner is disabled"
    );
}

/// A DB failure (`DatabaseFailure`) propagates: the caller rolls back and
/// backs off (no queue-side effect).
#[tokio::test]
async fn fail_event_propagate_bubbles_the_error() {
    let mut queue = MockQueue::new();
    queue
        .expect_provider_id()
        .returning(|| crate::Provider::Icu);
    queue.expect_disable_user().times(0);
    queue.expect_delete_event().times(0);
    let res = fail_event(
        &event(1, 1, 10),
        Error::DatabaseFailure(anyhow::anyhow!("db down")),
        &mut queue,
    )
    .await;
    assert!(matches!(res, Err(Error::DatabaseFailure(_))));
}

/// A permanent failure (a 404, a parse) drops the event: the delete commits
/// with the caller's transaction, and the loop continues.
#[tokio::test]
async fn fail_event_drop_event_deletes() {
    let mut queue = MockQueue::new();
    queue
        .expect_provider_id()
        .returning(|| crate::Provider::Icu);
    let deleted: std::sync::Arc<std::sync::Mutex<Option<Option<i32>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let rec = deleted.clone();
    queue.expect_delete_event().returning(move |id| {
        rec.lock().unwrap().replace(id);
        Ok(())
    });
    queue.expect_disable_user().times(0);
    let res = fail_event(&event(5, 1, 10), Error::NotFound("gone".into()), &mut queue)
        .await
        .unwrap();
    assert_eq!(res, Summary::default());
    assert_eq!(
        deleted.lock().unwrap().as_ref(),
        Some(&Some(5)),
        "the poison event is deleted"
    );
}
