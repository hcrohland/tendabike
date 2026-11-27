//! The thin intervals.icu HTTP calls: [`request_json`].
//!
//! Unlike the Strava driver there is no refresh machinery — intervals.icu
//! tokens are durable, so [`access_token`] reads the (durable) typed access
//! token straight from `icu_users` and [`request_json`] resolves it, then
//! makes the one bearer GET through the kit's shared request (the shared
//! offline seam, the bearer GET, the parse). A `401` (a revoked token) is
//! `Error::NotAuth` (the caller disables the user); a `429` / `5xx` is
//! `Error::TryAgain` (the rate-limit state is already updated from the
//! response headers, so the queue's probe reports the window boundary and
//! the loop sleeps until it).

use oauth2::AccessToken;
use serde::de::DeserializeOwned;

use super::{IcuId, IcuSpec};
use crate::facade::{Error, TbResult, UserId};
use crate::store::Store;

const API: &str = "https://intervals.icu/api/v1";

/// A valid access token for `user_id`: the (durable) typed token stored in
/// `icu_users`. No refresh — intervals.icu tokens never rotate.
async fn access_token(
    user_id: UserId,
    store: &mut impl Store<IcuSpec>,
) -> TbResult<(IcuId, AccessToken)> {
    let user = store.user_get_by_tbid(user_id).await?;
    let token = user
        .access_token
        .clone()
        .ok_or_else(|| Error::NotAuth(format!("no icu access token for user {user_id}")))?;
    Ok((user.id, token))
}

/// A bearer GET of `uri` for `user`, parsed as `T`: resolve the (durable)
/// token, then the kit's shared request.
pub(crate) async fn request_json<T: DeserializeOwned>(
    user_id: UserId,
    uri: &str,
    store: &mut impl Store<IcuSpec>,
) -> TbResult<T> {
    let (_icuid, access) = access_token(user_id, store).await?;
    crate::http::request_json::<T>(
        API,
        &access,
        uri,
        &super::ratelimit::update_from_headers,
        "intervals.icu",
    )
    .await
}

#[cfg(test)]
mod http_tests;
