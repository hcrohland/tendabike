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
//!
//! All operations return bare entities: a note operation touches only the note, so the write
//! contract requires no `Summary`. File bytes are fetched on demand via [`PartNote::file`],
//! never as part of any `Summary`.

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

/// A note or file attached to a part.
///
/// This struct is metadata only: for file entries the actual bytes are not stored here, they are
/// retrieved via the store. A note has a file attachment exactly when `mime` is set.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartNote {
    /// The primary key
    pub id: PartNoteId,
    /// The part this note belongs to
    pub part: PartId,
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

impl IdKeyed for PartNote {
    type Key = PartNoteId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

impl PartNote {
    /// Returns `true` when this note has a file attachment.
    pub fn has_file(&self) -> bool {
        self.mime.is_some()
    }

    /// Returns `true` when this note's mime type renders as an inline image.
    pub fn has_image(&self) -> bool {
        self.mime
            .as_deref()
            .is_some_and(|m| m.starts_with("image/"))
    }

    /// Retrieves the stored bytes of this note's file attachment.
    ///
    /// Returns `NotFound` for text notes (and for notes whose file data is missing).
    pub async fn file(&self, store: &mut impl PartNoteStore) -> TbResult<Vec<u8>> {
        if !self.has_file() {
            return Err(Error::NotFound(format!("note {} is not a file", self.id)));
        }
        store.partnote_file(self.id).await
    }
}

impl PartId {
    /// Returns all notes attached to this part, after checking access.
    pub async fn notes(
        self,
        user: &dyn Session,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<Vec<PartNote>> {
        self.checkuser(user, store).await?;
        store.partnote_all_by_part(self).await
    }

    /// Creates a free-text note on this part.
    ///
    /// Returns `BadRequest` for a blank name; access is checked against the part.
    pub async fn note_create_text(
        self,
        user: &dyn Session,
        name: String,
        created: OffsetDateTime,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        if name.trim().is_empty() {
            return Err(Error::BadRequest("note name must not be empty".to_string()));
        }
        self.checkuser(user, store).await?;
        store.partnote_create_text(self, name, created).await
    }

    /// Creates a file note on this part, storing the given bytes.
    #[allow(clippy::too_many_arguments)]
    pub async fn note_create_file(
        self,
        user: &dyn Session,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        created: OffsetDateTime,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        self.checkuser(user, store).await?;
        store
            .partnote_create_file(self, name, mime, filename, size, data, created)
            .await
    }
}

impl PartNoteId {
    /// Retrieves this note, after checking access to its part.
    pub async fn note(
        self,
        user: &dyn Session,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        let note = store.partnote_get(self).await?;
        note.part.checkuser(user, store).await?;
        Ok(note)
    }

    /// Updates the body of a text note.
    ///
    /// Returns `BadRequest` for a blank name; access is checked via [`PartNoteId::note`].
    pub async fn update_text(
        self,
        user: &dyn Session,
        name: String,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        if name.trim().is_empty() {
            return Err(Error::BadRequest("note name must not be empty".to_string()));
        }
        self.note(user, store).await?;
        store.partnote_update_text(self, name).await
    }

    /// Updates the file attachment of this note; converting a text note to a file note is
    /// allowed.
    ///
    /// `filename` falls back to the note's current filename (or `"file"`); `name` falls back
    /// to the resolved filename. Access is checked via [`PartNoteId::note`].
    #[allow(clippy::too_many_arguments)]
    pub async fn update_file(
        self,
        user: &dyn Session,
        name: Option<String>,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        let note = self.note(user, store).await?;
        let filename = filename
            .or_else(|| note.filename.clone())
            .unwrap_or_else(|| "file".to_string());
        let name = name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| filename.clone());
        store
            .partnote_update_file(self, name, mime, Some(filename), size, data)
            .await
    }

    /// Removes the file attachment, converting the note to a text note.
    ///
    /// Returns `NotFound` when the note has no file; access is checked via
    /// [`PartNoteId::note`].
    pub async fn remove_file(
        self,
        user: &dyn Session,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNote> {
        let note = self.note(user, store).await?;
        if !note.has_file() {
            return Err(Error::NotFound(format!("note {self} is not a file")));
        }
        store.partnote_remove_file(self).await
    }

    /// Deletes this note after checking access to its part.
    pub async fn delete(
        self,
        user: &dyn Session,
        store: &mut (impl PartNoteStore + PartStore + ShopStore),
    ) -> TbResult<PartNoteId> {
        self.note(user, store).await?;
        store.partnote_delete(self).await
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

#[cfg(test)]
mod partnote_tests;
