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

#[tokio::test]
async fn user_id_set_onboarding_status_delegates_to_store() -> crate::TbResult<()> {
    let mut store = crate::test_support::MockStore::new();
    let id = UserId::from(1i32);
    let updated = User {
        id,
        name: "Doe".into(),
        firstname: "John".into(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Completed,
    };
    store
        .expect_update_onboarding_status()
        .withf(move |uid, status| *uid == id && *status == OnboardingStatus::Completed)
        .times(1)
        .returning(move |_, _| Ok(updated.clone()));
    let result = id
        .set_onboarding_status(OnboardingStatus::Completed, &mut store)
        .await?;
    assert_eq!(result.onboarding_status, OnboardingStatus::Completed);
    Ok(())
}
