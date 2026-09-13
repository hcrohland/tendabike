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

//! This module contains the domain logic for part notes and file attachments.
//!
//! A [`PartNote`] is a single entry attached to a [`crate::Part`]. It is either a short free-text
//! note or an uploaded file. The in-memory/`PartNote` representation carries **metadata only**;
//! file bytes are fetched on demand through the store's `partnote_file` method.

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

/// The kind of a part note.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    /// A free-text note. `name` holds the note body.
    Text,
    /// An uploaded file. `name` holds the filename.
    File,
}

/// A note or file attached to a part.
///
/// This struct is metadata only: for [`NoteKind::File`] entries the actual bytes are not stored
/// here, they are retrieved via the store.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartNote {
    /// The primary key
    pub id: PartNoteId,
    /// The part this note belongs to
    pub part: PartId,
    /// Whether this is a text note or a file
    pub kind: NoteKind,
    /// Note body for text entries, filename for file entries
    pub name: String,
    /// MIME type (files only)
    pub mime: Option<String>,
    /// Original uploaded filename (files only)
    pub filename: Option<String>,
    /// Byte length of the file (files only)
    pub size: Option<i64>,
    /// When the note was created
    #[serde_as(as = "Rfc3339")]
    pub created: OffsetDateTime,
}

impl PartNote {
    /// Returns `true` when this note's mime type renders as an inline image.
    pub fn is_image(&self) -> bool {
        self.kind == NoteKind::File
            && self
                .mime
                .as_deref()
                .is_some_and(|m| m.starts_with("image/"))
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    From,
    Into,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
pub struct PartNoteId(i32);
