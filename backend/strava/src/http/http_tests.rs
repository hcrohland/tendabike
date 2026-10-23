//! Tests for the thin Strava HTTP free functions: the refresh-token
//! persistence (`get_token` → persist if `Some` → HTTP) is the one piece of
//! behavior the free functions own, so it is tested at the seam with a session
//! that reports a rotated refresh token.

use super::*;
use crate::StravaId;
use crate::test_support::{TestStravaSession, TestStravaStore, begin_test, queue, strava_user};
use tb_domain::{Session, ShopId, UserId};

/// A session whose `get_token` returns a warm dummy access token plus
/// (optionally) a new refresh token, so the free function's refresh-token
/// persistence is observable.
struct RefreshSession {
    user: UserId,
    strava: StravaId,
    new_refresh: Option<RefreshToken>,
}

impl Session for RefreshSession {
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

#[async_trait::async_trait]
impl StravaSession for RefreshSession {
    fn strava_id(&self) -> StravaId {
        self.strava
    }
    async fn get_token(&mut self) -> TbResult<(AccessToken, Option<RefreshToken>)> {
        Ok((
            AccessToken::new("test-access-token".to_owned()),
            self.new_refresh.clone(),
        ))
    }
}

fn rotated() -> RefreshSession {
    RefreshSession {
        user: UserId::from(1),
        strava: StravaId::from(42),
        new_refresh: Some(RefreshToken::new("rotated-token".to_owned())),
    }
}

/// `request_json` persists the rotated refresh token to the DB when `get_token`
/// returns `Some`.
#[tokio::test]
async fn request_json_persists_the_rotated_refresh_token() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    begin_test();
    queue("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let mut session = rotated();
    let _ath: serde_json::Value = request_json(&mut session, "/athlete", &mut store).await?;
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("rotated-token".to_string())
    );
    Ok(())
}

/// A warm session (`get_token` returns `None`) persists nothing: the stored
/// refresh token is left untouched.
#[tokio::test]
async fn request_json_warm_session_persists_nothing() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    begin_test();
    queue("/athlete", r#"{"bikes":[],"shoes":[]}"#);
    let mut session = RefreshSession {
        user: UserId::from(1),
        strava: StravaId::from(42),
        new_refresh: None,
    };
    let _ath: serde_json::Value = request_json(&mut session, "/athlete", &mut store).await?;
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("test-token".to_string())
    );
    Ok(())
}

/// `deauthorize` in test mode is recorded (no real revocation) and persists a
/// rotated refresh token when `get_token` returns `Some`.
#[tokio::test]
async fn deauthorize_records_and_persists() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    begin_test();
    let mut session = rotated();
    deauthorize(&mut session, &mut store).await?;
    assert_eq!(
        crate::test_support::deauthorizes(),
        vec![StravaId::from(42)]
    );
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("rotated-token".to_string())
    );
    Ok(())
}

/// The seam is object-safe: a session boxes as `dyn StravaSession` and its
/// `get_token` is callable through the trait object (the executor runs on a
/// `dyn StravaSession` from the next ticket on).
#[tokio::test]
async fn the_session_seam_is_object_safe() -> TbResult<()> {
    let mut session: Box<dyn StravaSession> =
        Box::new(TestStravaSession::new(UserId::from(1), StravaId::from(42)));
    let (access, new_refresh) = session.get_token().await?;
    assert_eq!(access.secret(), "test-access-token");
    assert!(new_refresh.is_none());
    Ok(())
}
