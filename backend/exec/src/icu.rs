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

//! The Icu provider's event payload (ticket #762) — a skeleton. The executor
//! is provider-agnostic: the [`Message::Icu`] variant carries the Icu
//! provider's event, parallel to [`Message::Strava`]'s
//! `tb_strava::event::Event`. The Icu provider is not yet wired (no `tb_icu`
//! crate, no `ProviderLane` implementation), so this module holds only the
//! event's payload shape: the minimum the `Message` enum needs to be
//! multi-provider today. When `tb_icu` is built, this type moves there and
//! the variant re-exports it, exactly as the `Strava` variant references
//! `tb_strava::event::Event`.

/// A skeleton event from the Icu provider: the payload [`Message::Icu`]
/// carries. The fields are the minimum an Icu webhook delivery is expected to
/// name (the activity it references and when it happened); the Icu provider
/// itself is not wired, so no code consumes them yet.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IcuEvent {
    /// The Icu delivery's id (the push subscription / delivery id), when
    /// present.
    pub id: Option<i32>,
    /// The activity the event references (the Icu activity id).
    pub activity_id: i64,
    /// The time the event occurred (unix seconds).
    pub event_time: i64,
}
