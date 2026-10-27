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

//! The `ApiRead` dispatch (ADR-0007, executable spec #558 §3).
//!
//! `ApiRead` is the one in-memory message an API read becomes on its way
//! through the per-user loop: a flat enum with one variant per per-user
//! read route, entity-prefixed, each variant carrying exactly the operation's
//! parsed arguments — the same values the current axum handlers extract. The
//! 15 per-user read routes collapse to the 12 catalog variants (spec #558
//! §3.4): `User { shop }` covers `GET /user`, `GET /user/summary`, and
//! `GET /user/export`, and `ShopSubscriptions(Option<ShopId>)` covers the
//! mine route and the shop's route. The admin `GET /user/all` is not a
//! variant: `get_all_stats` is a Strava-driver op, and the domain cannot
//! depend on the Strava driver (the dependency direction is
//! `domain ← strava`).
//!
//! No variant carries a `user_id`: the channel is per-user, the loop runs in
//! that user's context, and the `Session` the caller supplies carries the
//! identity.
//!
//! [`read`] applies one `ApiRead` through the domain layer — the same call
//! the current axum handler makes, minus the HTTP extraction and minus
//! `begin`/`commit`: the loop drives the
//! transaction (one loop-owned transaction per message, read-only within it
//! by spec), and `read` runs inside one live connection. It is bounded by
//! the `Store` marker, not by the caller's `Txn` trait, so it stays
//! verifiable on both adapters (`MemStore` and `SqlxConn`) per the
//! store-seam contract, with the Postgres behavior as the source of truth.
//!
//! The return is the read's data wrapped in a [`ReadOutcome`], a sealed enum
//! over the per-user read results: one variant per `ApiRead` variant, the
//! catalog's payload. The handler knows which variant it sent and
//! pattern-matches its own result; a mismatch is a bug, guarded the way
//! `write_descend` guards its report.

use std::collections::HashSet;

use crate::*;

/// One per-user read API route, as the per-user loop receives it.
///
/// The variants are the per-user read route table of `backend/axum`
/// (`src/domain/*.rs`), entity-prefixed. The `/types` routes are static (no
/// DB) and stay in the presentation layer. The admin routes are
/// deliberately absent: they are not per-user reads (spec #558 §3.3/§3.4).
#[derive(Clone, Debug, PartialEq)]
pub enum ApiRead {
    // --- user (`/api/user`) ---
    /// `GET /user`, `GET /user/summary` (after the hydrate write, spec
    /// #558 §4), and `GET /user/export` — the unified user read (spec
    /// #558 §3.5). The `?shop=` filter is the acting session's own shop
    /// (spec #517, ticket #588): `None` (personal mode) reads the user's
    /// whole state, `Some` filters the summary to the shop's parts. The
    /// routes project `User`, `Summary`, and the export composite from the
    /// one message.
    User,

    /// The light user-record read: just the [`User`] row, no `Summary`
    /// computation. Used by the onboarding read-backs (the webhook's
    /// `trigger_initial_sync` / `postpone_initial_sync`) which need only
    /// the updated record in their 200 body.
    UserRecord,

    // --- activity (`/api/activ`) ---
    /// `GET /activ/{id}`.
    Activity(ActivityId),

    // --- part (`/api/part`) ---
    /// `GET /part/{part}`.
    Part(PartId),
    /// `GET /part/categories` (no route arguments).
    PartCategories,
    /// `GET /part/{part}/notes`.
    PartNotes(PartId),

    // --- partnote (`/api/part`) ---
    /// `GET /part/notes/{id}/file` — the note plus its file bytes; the
    /// response headers/encoding stay in axum.
    PartNoteFile(PartNoteId),

    // --- shop (`/api/shop`) ---
    /// `GET /shop`.
    Shops,
    /// `GET /shop/{shop}`.
    Shop(ShopId),
    /// `GET /shop/{shop}/parts`.
    ShopParts(ShopId),
    /// `GET /shop/search` — the `?q=` query string.
    ShopSearch(String),
    /// `GET /shop/subscriptions` (mine, `None`) and
    /// `GET /shop/{shop}/subscriptions` (the shop's pending, `Some`).
    ShopSubscriptions(Option<ShopId>),
    /// `GET /shop/subscriptions/{id}`.
    ShopSubscription(SubscriptionId),
}

/// The outcome of one applied [`ApiRead`]: a sealed enum over the per-user
/// read results, one variant per `ApiRead` variant, carrying the catalog's
/// payload (spec #558 §3.4). Sealed: the set of variants is the catalog
/// itself — a new per-user read is a new `ApiRead` variant with its payload
/// here, and nothing outside the catalog may be returned. The handler knows
/// which variant it sent and pattern-matches its own result; a mismatch is
/// a bug, guarded the way `write_descend` guards its report.
#[derive(Clone, Debug, PartialEq)]
pub enum ReadOutcome {
    /// `ApiRead::User`: the user with its state — the routes project `User`,
    /// `Summary`, and the export composite from it. Boxed: the composite
    /// carries a full `Summary` (nine id-keyed maps), and the enum is an
    /// owned channel message — one allocation keeps the other payloads small.
    User(Box<UserState>),
    /// `ApiRead::UserRecord`: the bare user row (no `Summary`).
    UserRecord(User),
    /// `ApiRead::Activity`.
    Activity(Activity),
    /// `ApiRead::Part`.
    Part(Part),
    /// `ApiRead::PartCategories`.
    PartCategories(HashSet<PartTypeId>),
    /// `ApiRead::PartNotes`.
    PartNotes(Vec<PartNote>),
    /// `ApiRead::PartNoteFile`: the note plus its file bytes (headers and
    /// encoding are the presentation's concern).
    PartNoteFile(PartNote, Vec<u8>),
    /// `ApiRead::Shops`.
    Shops(Vec<Shop>),
    /// `ApiRead::Shop`.
    Shop(Shop),
    /// `ApiRead::ShopParts`.
    ShopParts(Vec<Part>),
    /// `ApiRead::ShopSearch`: the shops plus their owners' (and, for the
    /// caller's own shops, subscribers') public user records.
    ShopSearch {
        shops: Vec<Shop>,
        users: Vec<UserPublic>,
    },
    /// `ApiRead::ShopSubscriptions`.
    ShopSubscriptions(Vec<ShopSubscriptionWithDetails>),
    /// `ApiRead::ShopSubscription`.
    ShopSubscription(ShopSubscription),
}

/// Apply one [`ApiRead`] through the domain layer.
///
/// This is the single entry point the loop and the read handlers use to
/// apply an API read. It runs **inside** one live transaction — the loop
/// owns `begin`/`commit`/`rollback` — so it takes the session and the store
/// by mutable reference, exactly like the domain operations it calls, and it
/// never touches the transaction lifecycle itself. A read is read-only within that transaction by spec:
/// it computes and returns data, and it pushes no stream frame.
///
/// Each arm is the same call the current axum handler makes, minus the HTTP
/// extraction: path ids are already in their domain types, and the
/// access checks the handlers run are the domain ops' own.
///
/// Returns the read's data wrapped in a [`ReadOutcome`].
pub async fn read(
    read: ApiRead,
    session: &dyn Session,
    store: &mut impl Store,
) -> TbResult<ReadOutcome> {
    match read {
        // --- user ---
        // The unified user read (spec #558 §3.5): the `UserState` composite —
        // the full `User` plus the `Summary` of everything the user owns,
        // filtered by the acting session's shop (spec #517, ticket #588).
        ApiRead::User => {
            let state = session.user_id().read(session.shop(), store).await?;
            Ok(ReadOutcome::User(Box::new(state)))
        }
        // The light record-only read: just the `User` row, no `Summary`.
        ApiRead::UserRecord => {
            let user = UserStore::user_get(store, session.user_id()).await?;
            Ok(ReadOutcome::UserRecord(user))
        }

        // --- activity ---
        // The op checks authorization (the handler's `read` call).
        ApiRead::Activity(id) => Ok(ReadOutcome::Activity(id.read(session, store).await?)),

        // --- part ---
        // The op checks access: owner, or a shop session the user owns.
        ApiRead::Part(id) => Ok(ReadOutcome::Part(id.part(session, store).await?)),
        ApiRead::PartCategories => Ok(ReadOutcome::PartCategories(
            Part::categories(session, store).await?,
        )),

        // --- partnote ---
        // The ops check access to the note's part (the handler's calls).
        ApiRead::PartNotes(part) => Ok(ReadOutcome::PartNotes(part.notes(session, store).await?)),
        ApiRead::PartNoteFile(id) => {
            // The note, then its file bytes on demand — the download route's
            // data; headers and encoding stay in axum. A text note is a
            // NotFound (the op's rule).
            let note = id.note(session, store).await?;
            let data = note.file(store).await?;
            Ok(ReadOutcome::PartNoteFile(note, data))
        }

        // --- shop ---
        // The handler's calls, minus the HTTP extraction.
        ApiRead::Shops => Ok(ReadOutcome::Shops(
            Shop::get_all_for_user(&session.user_id(), store).await?,
        )),
        ApiRead::Shop(id) => Ok(ReadOutcome::Shop(id.read(store).await?)),
        ApiRead::ShopParts(id) => {
            // The route's read-access check (owner, or active subscriber),
            // then the op — which checks ownership itself.
            let id = ShopId::get_for_read(id.into(), session.user_id(), store).await?;
            Ok(ReadOutcome::ShopParts(
                id.get_parts(session.user_id(), store).await?,
            ))
        }
        ApiRead::ShopSearch(query) => {
            // The `?q=` query, the shops, and their owners' (and, for the
            // caller's own shops, subscribers') public records.
            let shops = Shop::search(&query, store).await?;
            let users = Shop::get_users(&shops, &session.user_id(), None, store).await?;
            Ok(ReadOutcome::ShopSearch { shops, users })
        }
        ApiRead::ShopSubscriptions(shop) => match shop {
            // The mine route: the session user's subscriptions, with shop
            // details.
            None => {
                let subs = ShopSubscription::get_for_user(session.user_id(), store).await?;
                Ok(ReadOutcome::ShopSubscriptions(
                    ShopSubscription::with_shop_details(subs, store).await?,
                ))
            }
            // The shop's route: the owner's check, then the shop's pending
            // subscriptions (the op checks ownership again itself).
            Some(shop) => {
                let shop = ShopId::get(shop.into(), session.user_id(), store).await?;
                Ok(ReadOutcome::ShopSubscriptions(
                    ShopSubscription::get_pending_for_shop(shop, session.user_id(), store).await?,
                ))
            }
        },
        ApiRead::ShopSubscription(id) => {
            // The access check (subscriber, or shop owner), then the read —
            // the handler's two calls.
            let id = SubscriptionId::get(id.into(), session.user_id(), store).await?;
            Ok(ReadOutcome::ShopSubscription(
                id.read(session.user_id(), store).await?,
            ))
        }
    }
}

#[cfg(test)]
mod apiread_tests;
