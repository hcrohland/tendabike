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


///! Domain model for tendabike
mod activity;
mod attachment;
mod error;
mod part;
mod presentation;
mod types;
mod user;

use async_session::log::{debug, info, trace, warn};
use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

pub use activity::{Activity, ActivityId, NewActivity};
pub use attachment::{Attachment, AttachmentDetail, Event};
use error::*;
pub use error::{AnyResult, Error};
pub use part::{ChangePart, NewPart, Part, PartId};
pub use presentation::Person;
pub use types::*;
pub use user::*;

use diesel::prelude::*;
use diesel::{self, QueryDsl};

use s_diesel::schema;
use s_diesel::AppConn;

mod usage;
pub use usage::*;

mod summary;
pub use summary::*;

mod traits;

mod store;
