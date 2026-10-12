//! In-memory test doubles for unit tests of the strava crate.
//!
//! `TestStravaSession` is a scripted `StravaSession` that replays
//! queued JSON responses for exact request URIs. The store double is
//! the shared `tb_teststore::TestStore` (spec #510): it doubles the
//! session trait and names no store, so it lives here.

use std::collections::{HashMap, VecDeque};

use oauth2::RefreshToken;
use serde::de::DeserializeOwned;

use crate::*;

/// Scripted [`StravaSession`] for tests.
///
/// `request_json` replays queued responses for the exact request URI and
/// records all requested URIs in `requests`.
pub struct TestStravaSession {
    user: UserId,
    strava: StravaId,
    admin: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    pub requests: Vec<String>,
    pub deauthorizes: Vec<StravaId>,
}

impl TestStravaSession {
    pub fn new(user: UserId, strava: StravaId) -> Self {
        Self {
            user,
            strava,
            admin: false,
            responses: HashMap::new(),
            requests: Vec::new(),
            deauthorizes: Vec::new(),
        }
    }

    /// Queue a JSON response body for an exact request URI.
    pub fn queue(&mut self, uri: &str, json: &str) {
        self.responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Ok(json.to_string()));
    }

    /// Queue an error for an exact request URI.
    pub fn queue_error(&mut self, uri: &str, err: Error) {
        self.responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Err(err));
    }
}

impl Session for TestStravaSession {
    fn user_id(&self) -> UserId {
        self.user
    }

    fn shop(&self) -> Option<ShopId> {
        None
    }

    fn set_shop(&mut self, _shop: Option<ShopId>) -> TbResult<()> {
        Ok(())
    }

    fn is_admin(&self) -> bool {
        self.admin
    }
}

#[async_trait::async_trait]
impl StravaSession for TestStravaSession {
    fn strava_id(&self) -> StravaId {
        self.strava
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaStore,
    ) -> TbResult<T> {
        self.requests.push(uri.to_string());
        let body = self
            .responses
            .get_mut(uri)
            .and_then(|q| q.pop_front())
            .unwrap_or(Err(Error::BadRequest(format!(
                "unexpected strava request: {uri}"
            ))));
        let body = body?;
        serde_json::from_str(&body).map_err(|e| Error::AnyFailure(e.into()))
    }

    async fn deauthorize(&mut self, _store: &mut impl StravaStore) -> TbResult<()> {
        self.deauthorizes.push(self.strava);
        Ok(())
    }
}

/// Builds a [`StravaUser`] for tests.
pub fn strava_user(tbid: UserId, stravaid: i32, enabled: bool) -> StravaUser {
    StravaUser {
        id: stravaid.into(),
        tendabike_id: tbid,
        refresh_token: enabled.then(|| RefreshToken::new("test-token".to_string())),
    }
}

/// Minimal Strava activity JSON with the given id, type, and gear id.
pub fn activity_json(id: i64, typ: &str, gear: Option<&str>) -> String {
    let gear = match gear {
        Some(g) => format!("\"{g}\""),
        None => "null".to_string(),
    };
    format!(
        r#"{{"id":{id},"type":"{typ}","name":"Test Ride","start_date":"2026-01-02T10:00:00Z","utc_offset":0,"elapsed_time":3600,"moving_time":3300,"distance":25000.0,"total_elevation_gain":300.0,"kilojoules":4000.0,"gear_id":{gear},"device_name":"Test Device","external_id":null}}"#
    )
}

/// Minimal Strava gear JSON with the given id; `frame_type` None means shoes.
pub fn gear_json(id: &str, frame_type: Option<i32>) -> String {
    let ft = frame_type
        .map(|t| t.to_string())
        .unwrap_or_else(|| "null".into());
    format!(
        r#"{{"id":"{id}","name":"Test Bike","brand_name":"Test Brand","model_name":"Test Model","frame_type":{ft}}}"#
    )
}
