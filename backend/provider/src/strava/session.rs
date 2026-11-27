/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! The Strava driver's token cache (spec #602): the process-wide
//! [`TokenCache`] the kit's generic refresh machinery
//! ([`get_token`](crate::token::get_token),
//! [`get_owner_id`](crate::token::get_owner_id)) drives.
//!
//! The DB keeps the refresh token as the durable fallback; on a cache miss
//! the kit's token state is built from the DB's refresh token (an empty
//! access token with a past expiry), forcing the first request to refresh.
//! The cache is a named per-provider static, reached by the kit by borrowing
//! a reference — the generic kit types stay state-free. No token type or
//! value crosses the crate boundary.

use std::sync::LazyLock;

use oauth2::AccessToken;

use super::{StravaId, StravaSpec};
use crate::TokenCache;
use crate::facade::{TbResult, UserId};
use crate::store::Store;

/// The process-wide Strava token cache (spec #602), keyed by [`UserId`].
pub(crate) static TOKEN_CACHE: LazyLock<TokenCache<StravaSpec>> = LazyLock::new(TokenCache::new);

/// A valid Strava access token for `user_id` (spec #602): the kit's generic
/// [`get_token`](crate::token::get_token), driven through the process-wide
/// cache. A refresh always stores the durable refresh token: when the
/// exchange issues a new rotating token it is persisted to the DB via the
/// store's `update_token` op before the access token is returned, so a
/// refresh can never happen without the rotated token being stored.
/// Returns `(strava_id, access)`.
pub(crate) async fn get_token(
    user_id: UserId,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<(StravaId, AccessToken)> {
    crate::token::get_token(user_id, &TOKEN_CACHE, store).await
}

/// The user's [`StravaId`] (spec #602): the kit's generic
/// [`get_owner_id`](crate::token::get_owner_id), driven through the
/// process-wide cache. A read-only owner resolution: it never refreshes.
/// Private to the Strava driver — no `strava_id` crosses the module boundary.
pub(crate) async fn get_owner_id(
    user_id: UserId,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<StravaId> {
    crate::token::get_owner_id(user_id, &TOKEN_CACHE, store).await
}
