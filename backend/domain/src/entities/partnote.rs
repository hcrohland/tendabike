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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MemStore;

    fn created() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
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
        assert_eq!(note.kind, NoteKind::Text);
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
        assert_eq!(note.kind, NoteKind::File);
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
        assert_eq!(updated.kind, NoteKind::Text);
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
        assert_eq!(updated.kind, NoteKind::File);
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
        assert_eq!(updated.kind, NoteKind::Text);
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
}
