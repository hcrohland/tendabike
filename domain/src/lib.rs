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

use async_session::log::{debug, info, trace, warn};
use diesel::prelude::*;

mod error;
pub use error::{Error, TbResult};

mod entities;
pub use entities::activity::{Activity, ActivityId, NewActivity};
pub use entities::attachment::{Attachment, AttachmentDetail, Event};
pub use entities::part::{ChangePart, NewPart, Part, PartId};
pub use entities::service::*;
// pub use entities::serviceplan::*;
pub use entities::summary::*;
pub use entities::types::*;
pub use entities::usage::*;
pub use entities::user::*;

pub mod schema;

mod traits;
pub use traits::*;

const MAX_TIME: time::OffsetDateTime = time::macros::datetime!(9100-01-01 0:00 UTC);
const MIN_TIME: time::OffsetDateTime = time::macros::datetime!(0000-01-01 0:00 UTC);
