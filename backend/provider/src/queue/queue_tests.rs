use super::{fail_event, get_event};
use crate::Provider;
use crate::ProviderSpec;
use crate::driver::Provider as Driver;
use crate::facade::{
    ActivityId, EntityFacade, Error, Part, PartId, Session, Summary, TbResult, UserId,
};
use crate::lane::QueueRead;
use crate::store::Store;
use crate::test_support::MockStore;
use oauth2::basic::BasicTokenResponse;

/// A queued event row for the queue-op tests: the identity the ops key on.
#[derive(Debug, Clone, PartialEq)]
struct TestEvent {
    id: Option<i32>,
    owner: i32,
    key: i64,
}

/// A test user row for the marker's user-row accessors: the TendaBike id and
/// the provider owner the accessors read.
#[derive(Debug, Clone)]
struct TestUser {
    tbid: UserId,
    owner: i32,
}

/// The test marker (spec #1029, ticket #1031): the type-only carrier of the
/// test event's wire shapes, so the queue's read and failure rules run over
/// the single [`MockStore`] double (generic over the marker).
struct TestSpec;

impl ProviderSpec for TestSpec {
    const PROVIDER: Provider = Provider::Icu;
    type User = TestUser;
    type Event = TestEvent;
    type Owner = i32;
    type Key = i64;
    type Driver = TestDriver;
    fn make_sync(owner: i32, _time: i64) -> TestEvent {
        TestEvent {
            id: None,
            owner,
            key: 0,
        }
    }
    fn event_id(e: &TestEvent) -> Option<i32> {
        e.id
    }
    fn event_owner(e: &TestEvent) -> &i32 {
        &e.owner
    }
    fn event_key(e: &TestEvent) -> &i64 {
        &e.key
    }
    fn user_tbid(user: &TestUser) -> &UserId {
        &user.tbid
    }
    fn user_owner(user: &TestUser) -> &i32 {
        &user.owner
    }
}

/// A test driver stub for the marker's `Driver` associated type (spec
/// #1062): the full driver trait — inert stubs for the provider-specific
/// core (never called from the queue ops) plus a dummy client
/// (locally-pointing auth/token URLs) — so the queue's read and failure
/// rules run over the single [`MockStore`] double without a real provider.
struct TestDriver;

#[async_trait::async_trait]
impl Driver<TestSpec> for TestDriver {
    type TokenResponse = BasicTokenResponse;
    type Client = crate::test_support::DummyClient<BasicTokenResponse>;

    /// The shared dummy oauth client (locally-pointing auth/token URLs):
    /// never called from the queue ops (the token hooks are the trait's
    /// defaults).
    fn client() -> &'static Self::Client {
        crate::test_support::basic_dummy_client()
    }

    // The provider-specific core: inert stubs, never called from the queue
    // ops (the read and failure rules drive the store only).

    fn new_user(id: i32, tbid: UserId, _token: Option<String>) -> TestUser {
        TestUser { tbid, owner: id }
    }

    fn disabled(_user: &TestUser) -> bool {
        false
    }

    fn authorize_url(_state: &str, _code_verifier: Option<&str>) -> String {
        "authorize".to_string()
    }

    async fn login<S: Store<TestSpec>, D: EntityFacade<S>>(
        _code: &str,
        _scope: &str,
        _verifier: Option<&str>,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<(UserId, bool)> {
        Ok((UserId::from(0), false))
    }

    async fn accept_event<S: Store<TestSpec>>(
        _body: &str,
        _store: &mut S,
    ) -> TbResult<Vec<UserId>> {
        Ok(Vec::new())
    }

    async fn import_gear<S: Store<TestSpec>, D: EntityFacade<S>>(
        _user_id: UserId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<Vec<PartId>> {
        Ok(Vec::new())
    }

    async fn process<S: Store<TestSpec>, D: EntityFacade<S>>(
        _user_id: UserId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<Summary> {
        Ok(Summary::default())
    }

    async fn activity_url<S: Store<TestSpec>, D: EntityFacade<S>>(
        _session: Box<dyn Session>,
        _id: ActivityId,
        _facade: &D,
        _store: &mut S,
    ) -> TbResult<String> {
        Ok("activity-url".to_string())
    }

    fn gear_url(_part: &Part) -> TbResult<Option<String>> {
        Ok(None)
    }

    async fn user_url<S: Store<TestSpec>>(_id: UserId, _store: &mut S) -> TbResult<String> {
        Ok("user-url".to_string())
    }
}

fn event(id: i32, owner: i32, key: i64) -> TestEvent {
    TestEvent {
        id: Some(id),
        owner,
        key,
    }
}

type MockQueue = MockStore<TestSpec>;

// ─── get_event (the read-side rules) ────────────────────────────────────────

/// The rate-limit gate fires before any store read: budget exhausted →
/// `Limited` (the event stays queued).
#[tokio::test]
async fn get_event_limited_when_rate_limited() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| Some(123));
    // The gate fires first: no store read.
    queue.expect_event_get_next().times(0);
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(matches!(read, QueueRead::Limited { until: 123 }));
}

/// An empty queue: no event for the owner → `Empty`.
#[tokio::test]
async fn get_event_empty_when_no_events() {
    let mut queue = MockQueue::new();
    queue.expect_limited_until().returning(|| None);
    queue.expect_event_get_next().returning(|_user| Ok(None));
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
        .expect_event_get_next()
        .returning(|_user| Ok(Some(event(1, 1, 10))));
    // The event is the only queued row for its key: the dedup no-ops.
    queue
        .expect_event_get_later()
        .returning(|_key, _owner| Ok(vec![event(1, 1, 10)]));
    queue.expect_event_delete_batch().times(0);
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
        .expect_event_get_next()
        .returning(|_user| Ok(Some(event(1, 1, 10))));
    queue
        .expect_event_get_later()
        .returning(|_key, _owner| Ok(vec![event(1, 1, 10), event(2, 1, 10)]));
    let dropped: std::sync::Arc<std::sync::Mutex<Vec<Option<i32>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let rec = dropped.clone();
    queue.expect_event_delete_batch().returning(move |values| {
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
        .expect_event_get_next()
        .returning(|_user| Ok(Some(event(7, 1, 999))));
    queue.expect_event_get_later().returning(|key, owner| {
        assert_eq!(key, 999, "keyed on the row's own key");
        assert_eq!(owner, 1, "keyed on the row's own owner");
        Ok(vec![event(7, 1, 999)])
    });
    queue.expect_event_delete_batch().times(0);
    let read = get_event(1, &mut queue).await.unwrap();
    assert!(
        matches!(read, QueueRead::Event(e) if e.id == Some(7)),
        "the row is returned as-is"
    );
}

// ─── fail_event (the four-arm policy) ───────────────────────────────────────

/// A rate limit (`TryAgain`) keeps the event queued: no store-side effect, an
/// empty summary (the next `get_event` probe reports `Limited`).
#[tokio::test]
async fn fail_event_keep_queued_is_a_noop() {
    let mut queue = MockQueue::new();
    queue.expect_user_disable().times(0);
    queue.expect_event_delete().times(0);
    let res = fail_event(&event(1, 1, 10), Error::TryAgain("rate limit"), &mut queue)
        .await
        .unwrap();
    assert_eq!(res, Summary::default());
}

/// Dead auth (`NotAuth`) disables the user (which drops their queued events):
/// the disable commits with the caller's transaction. The disable is a store
/// user-lifecycle op (`Store::user_disable`) (spec #1029, ticket #1031).
#[tokio::test]
async fn fail_event_disable_user_disables() {
    let mut queue = MockQueue::new();
    let disabled: std::sync::Arc<std::sync::Mutex<Option<i32>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let rec = disabled.clone();
    queue.expect_user_disable().returning(move |user| {
        rec.lock().unwrap().replace(user);
        Ok(())
    });
    queue.expect_event_delete().times(0);
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
/// backs off (no store-side effect).
#[tokio::test]
async fn fail_event_propagate_bubbles_the_error() {
    let mut queue = MockQueue::new();
    queue.expect_user_disable().times(0);
    queue.expect_event_delete().times(0);
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
    let deleted: std::sync::Arc<std::sync::Mutex<Option<Option<i32>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let rec = deleted.clone();
    queue.expect_event_delete().returning(move |id| {
        rec.lock().unwrap().replace(id);
        Ok(())
    });
    queue.expect_user_disable().times(0);
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
