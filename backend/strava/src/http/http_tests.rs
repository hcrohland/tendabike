//! Tests for the thin Strava HTTP free functions: the token now flows through
//! the `TokenCache` (spec #602), so the free functions are tested at the seam
//! with a warm token seeded into the `TokenCache`. A warm token returns `None`
//! for the new refresh token, so no refresh-token persistence happens.

use super::*;
use crate::StravaId;
use crate::StravaSession;
use crate::test_support::{
    TestStravaSession, TestStravaStore, begin_test, deauthorizes, queue, seed_token_cache,
    strava_user,
};
use tb_domain::{Session, ShopId, UserId};

/// A minimal `Session` for the free functions (the token now comes from the
/// `TokenCache`, not the session).
struct PlainSession {
    user: UserId,
}

impl Session for PlainSession {
    fn user_id(&self) -> UserId {
        self.user
    }
    fn shop(&self) -> Option<ShopId> {
        None
    }
    fn is_admin(&self) -> bool {
        false
    }
}

/// `request_json` uses the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token).
#[tokio::test]
async fn request_json_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = PlainSession {
        user: UserId::from(1),
    };
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    begin_test();
    queue("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let _ath: serde_json::Value = request_json(&mut session, "/athlete", &mut store).await?;
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}

/// `deauthorize` in test mode is recorded (no real revocation) and uses the
/// `TokenCache`'s warm token (persists nothing).
#[tokio::test]
async fn deauthorize_records_with_a_warm_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = PlainSession {
        user: UserId::from(1),
    };
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    begin_test();
    deauthorize(&mut session, &mut store).await?;
    assert_eq!(deauthorizes(), vec![StravaId::from(42)]);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}

/// The seam is object-safe: a session boxes as `dyn StravaSession` and its
/// `get_token` is callable through the trait object (the `StravaSession` trait
/// still exists in this step; it is removed in Step 3).
#[tokio::test]
async fn the_session_seam_is_object_safe() -> TbResult<()> {
    let mut session: Box<dyn StravaSession> =
        Box::new(TestStravaSession::new(UserId::from(1), StravaId::from(42)));
    let (access, new_refresh) = session.get_token().await?;
    assert_eq!(access.secret(), "test-access-token");
    assert!(new_refresh.is_none());
    Ok(())
}

/// `access_token` returns the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token).
#[tokio::test]
async fn access_token_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = PlainSession {
        user: UserId::from(1),
    };
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    let access = access_token(&mut session, &mut store).await?;
    assert_eq!(access.secret(), "test-access-token");
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}
