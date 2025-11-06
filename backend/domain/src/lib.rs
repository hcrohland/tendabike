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

mod traits;
use time::OffsetDateTime;
pub use traits::*;

const MAX_TIME: OffsetDateTime = time::macros::datetime!(9100-01-01 0:00 UTC);
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

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn test_round_time() {
        assert_eq!(
            round_time(datetime!(2020-01-01 0:00:00.0000 UTC)),
            datetime!(2020-01-01 0:00 UTC)
        );
        assert_eq!(
            round_time(datetime!(2020-01-01 0:07:07.0077 UTC)),
            datetime!(2020-01-01 0:00 UTC)
        );
        assert_eq!(
            round_time(datetime!(2020-02-28 0:15 -1)),
            datetime!(2020-02-28 0:15 -1)
        );
        assert_eq!(
            round_time(datetime!(2020-02-28 0:29:07.0077 -1)),
            datetime!(2020-02-28 0:15 -1)
        );
    }
}
