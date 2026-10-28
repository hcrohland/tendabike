//! Tests for the thin Strava HTTP free functions: the token now flows through
//! the `TokenCache` (spec #602), so the free functions are tested at the seam
//! with a warm token seeded into the `TokenCache`. A warm token returns `None`
//! for the new refresh token, so no refresh-token persistence happens: the
//! store witness is a bare `MockStravaConn` with no `stravauser_update_token`
//! expectation set — a refresh would call an unprogrammed mock method and
//! panic.

use super::*;
use crate::StravaId;
use crate::test_support::{
    MockStravaConn, begin_test, deauthorizes, queue, seed_token_cache, strava_session,
};
use tb_domain::UserId;

/// `request_json` uses the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token): the store witness
/// has no `stravauser_update_token` expectation, so a refresh would panic.
#[tokio::test]
async fn request_json_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    let mut session = strava_session(UserId::from(1), StravaId::from(42));
    queue("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let ath: serde_json::Value = request_json(&mut session, "/athlete", &mut store).await?;
    assert_eq!(ath["bikes"].as_array().unwrap().len(), 0);
    Ok(())
}

/// `deauthorize` in test mode is recorded (no real revocation) and uses the
/// `TokenCache`'s warm token (persists nothing).
#[tokio::test]
async fn deauthorize_records_with_a_warm_token() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    begin_test();
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    deauthorize(UserId::from(1), &mut store).await?;
    assert_eq!(deauthorizes(), vec![StravaId::from(42)]);
    Ok(())
}

/// `access_token` returns the `TokenCache`'s warm token and persists nothing
/// (a warm token returns `None` for the new refresh token).
#[tokio::test]
async fn access_token_uses_the_token_cache_warm_token() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    begin_test();
    seed_token_cache(UserId::from(1), StravaId::from(42), "test-access-token");
    let (_strava_id, access) = access_token(UserId::from(1), &mut store).await?;
    assert_eq!(access.secret(), "test-access-token");
    Ok(())
}
