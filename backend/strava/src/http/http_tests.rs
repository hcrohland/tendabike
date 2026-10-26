//! Tests for the thin Strava HTTP free functions: the token now flows through
//! the `TokenCache` (spec #602), so the free functions are tested at the seam
//! with a warm token seeded into the `TokenCache`. A warm token returns `None`
//! for the new refresh token, so no refresh-token persistence happens.

use super::*;
use crate::StravaId;
use crate::test_support::{
    TestStravaStore, begin_test, deauthorizes, queue, seed_token_cache, strava_user,
};
use tb_domain::UserId;
use tb_domain::test_support::mock_session;

/// `request_json` uses the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token).
#[tokio::test]
async fn request_json_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = mock_session(UserId::from(1));
    begin_test();
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
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
    begin_test();
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    deauthorize(UserId::from(1), &mut store).await?;
    assert_eq!(deauthorizes(), vec![StravaId::from(42)]);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}

/// `access_token` returns the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token).
#[tokio::test]
async fn access_token_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    begin_test();
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    let (_strava_id, access) = access_token(UserId::from(1), &mut store).await?;
    assert_eq!(access.secret(), "test-access-token");
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}
