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

mod activity;
mod attachment;
mod error;
mod part;
mod types;
mod user;

use async_session::log::{debug, info, trace, warn};

pub use activity::{Activity, ActivityId, NewActivity};
pub use attachment::{Attachment, AttachmentDetail, Event};
pub use error::{Error, TbResult};
pub use part::{ChangePart, NewPart, Part, PartId};
pub use types::*;
pub use user::*;

use diesel::prelude::*;

pub mod schema;

mod usage;
pub use usage::*;

mod summary;
pub use summary::*;

mod traits;
pub use traits::*;
