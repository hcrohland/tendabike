use super::*;

#[test]
fn onboarding_status_try_from_valid() {
    assert_eq!(
        OnboardingStatus::try_from(0i32).unwrap(),
        OnboardingStatus::Pending
    );
    assert_eq!(
        OnboardingStatus::try_from(2i32).unwrap(),
        OnboardingStatus::InitialSyncPostponed
    );
    assert_eq!(
        OnboardingStatus::try_from(99i32).unwrap(),
        OnboardingStatus::Completed
    );
}

#[test]
fn onboarding_status_try_from_invalid() {
    assert!(matches!(
        OnboardingStatus::try_from(1i32).unwrap_err(),
        Error::BadRequest(_)
    ));
    assert!(matches!(
        OnboardingStatus::try_from(5i32).unwrap_err(),
        Error::BadRequest(_)
    ));
    assert!(matches!(
        OnboardingStatus::try_from(-1i32).unwrap_err(),
        Error::BadRequest(_)
    ));
}

#[test]
fn onboarding_status_from_i32_roundtrip() {
    for status in [
        OnboardingStatus::Pending,
        OnboardingStatus::InitialSyncPostponed,
        OnboardingStatus::Completed,
    ] {
        let val = i32::from(status);
        assert_eq!(OnboardingStatus::try_from(val).unwrap(), status);
    }
}

#[test]
fn onboarding_status_is_initial_sync_completed() {
    assert!(!OnboardingStatus::Pending.is_initial_sync_completed());
    assert!(!OnboardingStatus::InitialSyncPostponed.is_initial_sync_completed());
    assert!(OnboardingStatus::Completed.is_initial_sync_completed());
}

#[test]
fn user_public_drops_admin_and_onboarding() {
    let user = User {
        id: UserId::from(1i32),
        name: "Doe".into(),
        firstname: "John".into(),
        avatar: Some("http://example.com/a.png".into()),
        is_admin: true,
        onboarding_status: OnboardingStatus::Completed,
    };
    let pub_ = UserPublic::from(user);
    let json = serde_json::to_string(&pub_).unwrap();
    assert!(!json.contains("is_admin"));
    assert!(!json.contains("onboarding_status"));
    assert!(json.contains("\"name\":\"Doe\""));
    assert!(json.contains("\"firstname\":\"John\""));
}

#[test]
fn user_id_check_owner_same() {
    let id = UserId::from(42i32);
    assert!(id.check_owner(UserId::from(42i32), "nope".into()).is_ok());
}

#[test]
fn user_id_check_owner_different() {
    let id = UserId::from(42i32);
    let err = id
        .check_owner(UserId::from(43i32), "not your resource".into())
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
}

// === Tier B: User CRUD (in-memory store) ===

use crate::test_support::MemStore;

#[tokio::test]
async fn user_create_and_read() {
    let mut store = MemStore::new();
    let uid = UserId::create("John", "Doe", &None, &mut store)
        .await
        .unwrap();
    let user = uid.read(&mut store).await.unwrap();
    assert_eq!(user.id, uid);
    assert_eq!(user.firstname, "John");
    assert_eq!(user.name, "Doe");
    assert!(!user.is_admin);
    assert_eq!(user.onboarding_status, OnboardingStatus::Pending);
}

#[tokio::test]
async fn user_get_public() {
    let mut store = MemStore::new();
    let uid = UserId::create("Jane", "Smith", &Some("avatar.png".into()), &mut store)
        .await
        .unwrap();
    let pub_ = uid.get_public(&mut store).await.unwrap();
    assert_eq!(pub_.id, uid);
    assert_eq!(pub_.firstname, "Jane");
    assert_eq!(pub_.name, "Smith");
    assert_eq!(pub_.avatar, Some("avatar.png".into()));
}

#[tokio::test]
async fn user_update() {
    let mut store = MemStore::new();
    let uid = UserId::create("Alice", "Bob", &None, &mut store)
        .await
        .unwrap();
    uid.update("Alice", "NewName", &Some("new.png".into()), &mut store)
        .await
        .unwrap();
    let user = uid.read(&mut store).await.unwrap();
    assert_eq!(user.firstname, "Alice");
    assert_eq!(user.name, "NewName");
    assert_eq!(user.avatar, Some("new.png".into()));
}

#[tokio::test]
async fn user_is_admin_false_by_default() {
    let mut store = MemStore::new();
    let uid = UserId::create("Bob", "Builder", &None, &mut store)
        .await
        .unwrap();
    assert!(!uid.is_admin(&mut store).await.unwrap());
}

#[tokio::test]
async fn user_delete_removes_from_store() {
    let mut store = MemStore::new();
    let uid = UserId::create("Del", "User", &None, &mut store)
        .await
        .unwrap();
    uid.delete(&mut store).await.unwrap();
    assert!(uid.read(&mut store).await.is_err());
}

#[tokio::test]
async fn user_update_onboarding_status() {
    let mut store = MemStore::new();
    let uid = UserId::create("Carol", "C", &None, &mut store)
        .await
        .unwrap();
    store
        .update_onboarding_status(&uid, OnboardingStatus::Completed)
        .await
        .unwrap();
    let user = uid.read(&mut store).await.unwrap();
    assert_eq!(user.onboarding_status, OnboardingStatus::Completed);
}

// === Tier C: User summary (prepopulated fixture) ===

use crate::test_support::part_type_ids::BIKE;

/// The user summary read returns the full prepopulated fixture content
/// — the in-memory twin of the store-seam suite's `user_summary_read`
/// (which asserts the same thing over the same fixture loaded into
/// real Postgres).
#[tokio::test]
async fn user_summary_read() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let summary = UserId::from(1).get_summary(None, &mut store).await?;

    // The fixture content. Parts come back in one unified order on
    // both stores — ascending `last_used` (#405) — but the summary map
    // is unordered, so the assertion below sorts the part values by
    // `last_used` and checks the rule on the sorted sequence. See the
    // seam's `user_summary_read`.
    assert_eq!(summary.parts.len(), 17);
    let mut parts = summary.parts.live();
    parts.sort_by_key(|p| p.last_used);
    for (earlier, later) in parts.iter().zip(parts.iter().skip(1)) {
        assert!(
            earlier.last_used <= later.last_used,
            "parts not sorted by last_used"
        );
    }
    assert_eq!(summary.activities.len(), 3);
    assert_eq!(summary.attachments.len(), 11);
    // 17 part usages + 11 attachment usages (missing usage rows read
    // as zeros, identically on both stores).
    assert_eq!(summary.usages.len(), 28);
    assert!(summary.shops.is_empty());
    assert!(summary.users.is_empty());
    assert!(summary.services.is_empty());
    assert!(summary.plans.is_empty());
    assert!(summary.part_notes.is_empty());

    // Field lookups: the "Chain A" part, its attachment, and its usage.
    let chain = summary
        .parts
        .live()
        .into_iter()
        .find(|p| p.id == PartId::from(4))
        .unwrap();
    assert_eq!(chain.name, "Chain A");
    let att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == PartId::from(4))
        .unwrap();
    assert_eq!(att.a.hook, BIKE);
    assert_eq!(att.a.detached, MAX_TIME);
    let usage = summary
        .usages
        .live()
        .into_iter()
        .find(|u| u.id == chain.usage)
        .unwrap();
    assert_eq!(usage.time, 8025);
    assert_eq!(usage.count, 3);

    Ok(())
}
