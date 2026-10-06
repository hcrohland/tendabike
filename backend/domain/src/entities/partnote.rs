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
//! contract requires no `SummaryVec`. File bytes are fetched on demand via [`PartNote::file`],
//! never as part of any `SummaryVec`.

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
mod tests {
    use super::*;
    use crate::test_support::{MemStore, TestSession};

    fn created() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    /// A fresh store with a bike part owned by user 1.
    async fn owned_part() -> (MemStore, PartId) {
        let mut store = MemStore::new();
        let owner = TestSession::new(UserId::from(1));
        let part = Part::create(
            "Bike".to_string(),
            "Trek".to_string(),
            "Domane".to_string(),
            PartTypeId::from(1),
            None,
            created(),
            &owner,
            &mut store,
        )
        .await
        .unwrap();
        (store, part.id)
    }

    fn other_user() -> TestSession {
        TestSession::new(UserId::from(98))
    }

    fn file_args() -> (String, String, Option<String>, i64, Vec<u8>) {
        (
            "photo".to_string(),
            "image/png".to_string(),
            Some("photo.png".to_string()),
            1234,
            vec![1, 2, 3],
        )
    }

    /// partnote_create_text stores a text note with the file fields left empty
    #[tokio::test]
    async fn partnote_create_text_stores_metadata_only() -> TbResult<()> {
        let mut store = MemStore::new();
        let note = store
            .partnote_create_text(PartId::from(1), "suspicious noise".to_string(), created())
            .await?;
        assert_eq!(note.id, PartNoteId::from(1));
        assert_eq!(note.part, PartId::from(1));
        assert!(!note.has_file());
        assert_eq!(note.name, "suspicious noise");
        assert_eq!(note.mime, None);
        assert_eq!(note.filename, None);
        assert_eq!(note.size, None);
        assert_eq!(note.created, created());
        Ok(())
    }

    /// partnote_create_file stores the metadata and the file data
    #[tokio::test]
    async fn partnote_create_file_stores_metadata_and_data() -> TbResult<()> {
        let mut store = MemStore::new();
        let (name, mime, filename, size, data) = file_args();
        let note = store
            .partnote_create_file(
                PartId::from(1),
                name,
                mime,
                filename,
                size,
                data.clone(),
                created(),
            )
            .await?;
        assert_eq!(note.id, PartNoteId::from(1));
        assert!(note.has_file());
        assert_eq!(note.mime, Some("image/png".to_string()));
        assert_eq!(note.filename, Some("photo.png".to_string()));
        assert_eq!(note.size, Some(1234));
        assert_eq!(store.partnote_file(note.id).await?, data);
        Ok(())
    }

    /// partnote_all_by_part returns only the notes of the given part
    #[tokio::test]
    async fn partnote_all_by_part_filters_by_part() -> TbResult<()> {
        let mut store = MemStore::new();
        store
            .partnote_create_text(PartId::from(1), "a".to_string(), created())
            .await?;
        store
            .partnote_create_text(PartId::from(2), "b".to_string(), created())
            .await?;
        let notes = store.partnote_all_by_part(PartId::from(1)).await?;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].name, "a");
        Ok(())
    }

    /// partnote_get returns an error for an unknown note
    #[tokio::test]
    async fn partnote_get_unknown_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = store.partnote_get(PartNoteId::from(99)).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_file returns an error for a text note without file data
    #[tokio::test]
    async fn partnote_file_of_text_note_errors() -> TbResult<()> {
        let mut store = MemStore::new();
        let note = store
            .partnote_create_text(PartId::from(1), "a".to_string(), created())
            .await?;
        let result = store.partnote_file(note.id).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_update_text renames a note
    #[tokio::test]
    async fn partnote_update_text_renames_note() -> TbResult<()> {
        let mut store = MemStore::new();
        let note = store
            .partnote_create_text(PartId::from(1), "old".to_string(), created())
            .await?;
        let updated = store
            .partnote_update_text(note.id, "new".to_string())
            .await?;
        assert_eq!(updated.name, "new");
        assert!(!updated.has_file());
        Ok(())
    }

    /// partnote_update_text returns an error for an unknown note
    #[tokio::test]
    async fn partnote_update_text_unknown_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = store
            .partnote_update_text(PartNoteId::from(99), "x".to_string())
            .await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_update_file converts a text note into a file note
    #[tokio::test]
    async fn partnote_update_file_converts_text_to_file() -> TbResult<()> {
        let mut store = MemStore::new();
        let note = store
            .partnote_create_text(PartId::from(1), "a".to_string(), created())
            .await?;
        let (name, mime, filename, size, data) = file_args();
        let updated = store
            .partnote_update_file(note.id, name, mime, filename, size, data.clone())
            .await?;
        assert!(updated.has_file());
        assert_eq!(updated.mime, Some("image/png".to_string()));
        assert_eq!(updated.size, Some(1234));
        assert_eq!(store.partnote_file(note.id).await?, data);
        Ok(())
    }

    /// partnote_update_file returns an error for an unknown note
    #[tokio::test]
    async fn partnote_update_file_unknown_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let (name, mime, filename, size, data) = file_args();
        let result = store
            .partnote_update_file(PartNoteId::from(99), name, mime, filename, size, data)
            .await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_remove_file converts a file note into a text note and drops the data
    #[tokio::test]
    async fn partnote_remove_file_converts_file_to_text() -> TbResult<()> {
        let mut store = MemStore::new();
        let (name, mime, filename, size, data) = file_args();
        let note = store
            .partnote_create_file(PartId::from(1), name, mime, filename, size, data, created())
            .await?;
        let updated = store.partnote_remove_file(note.id).await?;
        assert!(!updated.has_file());
        assert_eq!(updated.mime, None);
        assert_eq!(updated.filename, None);
        assert_eq!(updated.size, None);
        let result = store.partnote_file(note.id).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_remove_file returns an error for an unknown note
    #[tokio::test]
    async fn partnote_remove_file_unknown_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = store.partnote_remove_file(PartNoteId::from(99)).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// partnote_delete removes the note and its file data
    #[tokio::test]
    async fn partnote_delete_removes_note_and_file_data() -> TbResult<()> {
        let mut store = MemStore::new();
        let (name, mime, filename, size, data) = file_args();
        let note = store
            .partnote_create_file(PartId::from(1), name, mime, filename, size, data, created())
            .await?;
        assert_eq!(store.partnote_delete(note.id).await?, note.id);
        let result = store.partnote_get(note.id).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        let result = store.partnote_file(note.id).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        let notes = store.partnote_all_by_part(PartId::from(1)).await?;
        assert!(notes.is_empty());
        Ok(())
    }

    /// partnote_delete returns an error for an unknown note
    #[tokio::test]
    async fn partnote_delete_unknown_returns_not_found() -> TbResult<()> {
        let mut store = MemStore::new();
        let result = store.partnote_delete(PartNoteId::from(99)).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    // === Domain operations (Tier B: single-entity CRUD + validation) ===

    /// note_create_text rejects a blank name
    #[tokio::test]
    async fn partid_note_create_text_rejects_blank_name() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let result = part
            .note_create_text(&owner, "   ".to_string(), created(), &mut store)
            .await;
        assert!(matches!(result, Err(Error::BadRequest(_))));
        Ok(())
    }

    /// note_create_text stores the note on the part
    #[tokio::test]
    async fn partid_note_create_text_stores_note() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(
                &owner,
                "suspicious noise".to_string(),
                created(),
                &mut store,
            )
            .await?;
        assert_eq!(note.part, part);
        assert_eq!(note.name, "suspicious noise");
        assert!(!note.has_file());
        assert_eq!(note.created, created());
        Ok(())
    }

    /// note_create_file stores the metadata and the file data
    #[tokio::test]
    async fn partid_note_create_file_stores_file() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let (name, mime, filename, size, data) = file_args();
        let note = part
            .note_create_file(
                &owner,
                name,
                mime,
                filename.clone(),
                size,
                data.clone(),
                created(),
                &mut store,
            )
            .await?;
        assert!(note.has_file());
        assert_eq!(note.filename, filename);
        assert_eq!(note.size, Some(size));
        assert_eq!(note.file(&mut store).await?, data);
        Ok(())
    }

    /// notes returns all notes of the part
    #[tokio::test]
    async fn partid_notes_returns_part_notes() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        part.note_create_text(&owner, "a".to_string(), created(), &mut store)
            .await?;
        let (name, mime, filename, size, data) = file_args();
        part.note_create_file(
            &owner,
            name,
            mime,
            filename,
            size,
            data,
            created(),
            &mut store,
        )
        .await?;
        part.note_create_text(&owner, "c".to_string(), created(), &mut store)
            .await?;
        let notes = part.notes(&owner, &mut store).await?;
        assert_eq!(notes.len(), 3);
        let names: Vec<&str> = notes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec!["a", "c", "photo"]);
        Ok(())
    }

    /// update_text renames the note
    #[tokio::test]
    async fn partnote_id_update_text_renames_note() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "old".to_string(), created(), &mut store)
            .await?;
        let updated = note
            .id
            .update_text(&owner, "new".to_string(), &mut store)
            .await?;
        assert_eq!(updated.name, "new");
        assert!(!updated.has_file());
        Ok(())
    }

    /// update_text rejects a blank name
    #[tokio::test]
    async fn partnote_id_update_text_rejects_blank_name() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "old".to_string(), created(), &mut store)
            .await?;
        let result = note
            .id
            .update_text(&owner, "  ".to_string(), &mut store)
            .await;
        assert!(matches!(result, Err(Error::BadRequest(_))));
        Ok(())
    }

    /// update_file converts a text note into a file note
    #[tokio::test]
    async fn partnote_id_update_file_converts_text_to_file() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "a".to_string(), created(), &mut store)
            .await?;
        let (name, mime, filename, size, data) = file_args();
        let updated = note
            .id
            .update_file(
                &owner,
                Some(name),
                mime,
                filename.clone(),
                size,
                data.clone(),
                &mut store,
            )
            .await?;
        assert!(updated.has_file());
        assert_eq!(updated.filename, filename);
        assert_eq!(updated.size, Some(size));
        assert_eq!(updated.file(&mut store).await?, data);
        Ok(())
    }

    /// update_file keeps the current filename and name when the multipart sends neither
    #[tokio::test]
    async fn partnote_id_update_file_falls_back_to_existing_filename() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "a".to_string(), created(), &mut store)
            .await?;
        let (name, mime, filename, size, _) = file_args();
        note.id
            .update_file(
                &owner,
                Some(name),
                mime,
                filename.clone(),
                size,
                vec![1],
                &mut store,
            )
            .await?;
        let updated = note
            .id
            .update_file(
                &owner,
                None,
                "image/png".to_string(),
                None,
                1,
                vec![2, 2],
                &mut store,
            )
            .await?;
        assert_eq!(updated.filename, filename);
        assert_eq!(updated.name, "photo.png");
        assert_eq!(updated.file(&mut store).await?, vec![2, 2]);
        Ok(())
    }

    /// remove_file converts a file note into a text note
    #[tokio::test]
    async fn partnote_id_remove_file_converts_file_to_text() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let (name, mime, filename, size, data) = file_args();
        let note = part
            .note_create_file(
                &owner,
                name,
                mime,
                filename,
                size,
                data,
                created(),
                &mut store,
            )
            .await?;
        let updated = note.id.remove_file(&owner, &mut store).await?;
        assert!(!updated.has_file());
        assert_eq!(updated.filename, None);
        let result = updated.file(&mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// remove_file on a text note is NotFound
    #[tokio::test]
    async fn partnote_id_remove_file_of_text_note_is_not_found() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "a".to_string(), created(), &mut store)
            .await?;
        let result = note.id.remove_file(&owner, &mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// file returns the stored bytes
    #[tokio::test]
    async fn partnote_file_returns_stored_bytes() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let (name, mime, filename, size, data) = file_args();
        let note = part
            .note_create_file(
                &owner,
                name,
                mime,
                filename,
                size,
                data.clone(),
                created(),
                &mut store,
            )
            .await?;
        assert_eq!(note.file(&mut store).await?, data);
        Ok(())
    }

    /// file on a text note is NotFound
    #[tokio::test]
    async fn partnote_file_of_text_note_is_not_found() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let note = part
            .note_create_text(&owner, "a".to_string(), created(), &mut store)
            .await?;
        let result = note.file(&mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// delete removes the note and its file data
    #[tokio::test]
    async fn partnote_id_delete_removes_note() -> TbResult<()> {
        let (mut store, part) = owned_part().await;
        let owner = TestSession::new(UserId::from(1));
        let (name, mime, filename, size, data) = file_args();
        let note = part
            .note_create_file(
                &owner,
                name,
                mime,
                filename,
                size,
                data,
                created(),
                &mut store,
            )
            .await?;
        assert_eq!(note.id.delete(&owner, &mut store).await?, note.id);
        let result = note.id.note(&owner, &mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        let notes = part.notes(&owner, &mut store).await?;
        assert!(notes.is_empty());
        Ok(())
    }

    // === Domain operations (Tier C: permission and cross-entity) ===

    /// notes on an unknown part is NotFound
    #[tokio::test]
    async fn partid_notes_of_unknown_part_is_not_found() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let result = PartId::from(99).notes(&session, &mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// notes of another user's part is Forbidden
    #[tokio::test]
    async fn partid_notes_of_foreign_part_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let result = PartId::from(1).notes(&other_user(), &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// note for an unknown id is NotFound
    #[tokio::test]
    async fn partnote_id_note_of_unknown_id_is_not_found() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let result = PartNoteId::from(99).note(&session, &mut store).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
        Ok(())
    }

    /// note of another user's part is Forbidden
    #[tokio::test]
    async fn partnote_id_note_of_foreign_part_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let note = PartId::from(1)
            .note_create_text(&session, "a".to_string(), created(), &mut store)
            .await?;
        let result = note.id.note(&other_user(), &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// note_create_text of another user's part is Forbidden
    #[tokio::test]
    async fn partid_note_create_text_on_foreign_part_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let result = PartId::from(1)
            .note_create_text(&other_user(), "a".to_string(), created(), &mut store)
            .await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// note_create_file of another user's part is Forbidden
    #[tokio::test]
    async fn partid_note_create_file_on_foreign_part_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let (name, mime, filename, size, data) = file_args();
        let result = PartId::from(1)
            .note_create_file(
                &other_user(),
                name,
                mime,
                filename,
                size,
                data,
                created(),
                &mut store,
            )
            .await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// update_text of another user's note is Forbidden
    #[tokio::test]
    async fn partnote_id_update_text_of_foreign_note_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let note = PartId::from(1)
            .note_create_text(&session, "a".to_string(), created(), &mut store)
            .await?;
        let result = note
            .id
            .update_text(&other_user(), "b".to_string(), &mut store)
            .await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// update_file of another user's note is Forbidden
    #[tokio::test]
    async fn partnote_id_update_file_of_foreign_note_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let note = PartId::from(1)
            .note_create_text(&session, "a".to_string(), created(), &mut store)
            .await?;
        let (name, mime, filename, size, data) = file_args();
        let result = note
            .id
            .update_file(
                &other_user(),
                Some(name),
                mime,
                filename,
                size,
                data,
                &mut store,
            )
            .await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// remove_file of another user's note is Forbidden
    #[tokio::test]
    async fn partnote_id_remove_file_of_foreign_note_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let (name, mime, filename, size, data) = file_args();
        let note = PartId::from(1)
            .note_create_file(
                &session,
                name,
                mime,
                filename,
                size,
                data,
                created(),
                &mut store,
            )
            .await?;
        let result = note.id.remove_file(&other_user(), &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// delete of another user's note is Forbidden
    #[tokio::test]
    async fn partnote_id_delete_of_foreign_note_is_forbidden() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let session = TestSession::new(UserId::from(1));
        let note = PartId::from(1)
            .note_create_text(&session, "a".to_string(), created(), &mut store)
            .await?;
        let result = note.id.delete(&other_user(), &mut store).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
        Ok(())
    }

    /// a shop owner with a session on the registered part can run the note operations
    #[tokio::test]
    async fn shop_owner_can_access_partnotes() -> TbResult<()> {
        let mut store = MemStore::prepopulated();
        let shop_owner = UserId::create("Shop", "Owner", &None, &mut store).await?;
        let shop =
            ShopId::create("Bike Barn".to_string(), None, true, shop_owner, &mut store).await?;
        let sub = SubscriptionId::create(shop.id, None, UserId::from(1), &mut store).await?;
        assert_eq!(sub.status, SubscriptionStatus::Active);
        let customer = TestSession::new(UserId::from(1));
        shop.id
            .register_part(PartId::from(1), &customer, &mut store)
            .await?;

        let shop_session = TestSession::with_shop(shop_owner, shop.id);
        let part = PartId::from(1);
        let note = part
            .note_create_text(
                &shop_session,
                "check chain".to_string(),
                created(),
                &mut store,
            )
            .await?;
        let notes = part.notes(&shop_session, &mut store).await?;
        assert_eq!(notes.len(), 1);
        let updated = note
            .id
            .update_text(&shop_session, "chain worn".to_string(), &mut store)
            .await?;
        assert_eq!(updated.name, "chain worn");
        assert_eq!(note.id.delete(&shop_session, &mut store).await?, note.id);
        let notes = part.notes(&shop_session, &mut store).await?;
        assert!(notes.is_empty());
        Ok(())
    }
}
