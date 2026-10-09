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

use log::{debug, info, trace, warn};

mod error;
pub use error::{Error, TbResult};

mod entities;
pub use entities::*;

mod apiwrite;
pub use apiwrite::*;

mod traits;
use time::OffsetDateTime;
pub use traits::*;

/// The sentinel "still attached"/"never redone" time (year 9100): an
/// attachment with `detached == MAX_TIME` is still attached, and a service
/// with `redone == MAX_TIME` is still valid — the open-ended bound of a span
/// that never closed (see `entities/ATTACHMENT.md` and `entities/SERVICE.md`).
///
/// `pub` because it is domain vocabulary that production code (attachment
/// and service logic) and tests assert against — including the external
/// store-seam integration suite (`tb_sqlx`'s `tests/store_seam.rs`), a
/// separate crate that can only import public items. `MIN_TIME` has no such
/// external consumers and stays private.
pub const MAX_TIME: OffsetDateTime = time::macros::datetime!(9100-01-01 0:00 UTC);
/// The opposite bound (year 0): the "from the very beginning" end of a span,
/// as in a service's usage window `[MIN_TIME, service.time]` (`entities/SERVICE.md`).
/// Used only inside this crate, so it stays private.
const MIN_TIME: OffsetDateTime = time::macros::datetime!(0000-01-01 0:00 UTC);

/// round time down to the quarter of an hour
///
/// # Panics
///
/// Panics if the rounding leads to a ComponentRange error
pub fn round_time(time: OffsetDateTime) -> OffsetDateTime {
    let minute = time.minute();
    time.replace_microsecond(0)
        .unwrap()
        .replace_millisecond(0)
        .unwrap()
        .replace_second(0)
        .unwrap()
        .replace_minute((minute / 15) * 15)
        .unwrap()
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
