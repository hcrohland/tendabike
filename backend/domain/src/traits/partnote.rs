use time::OffsetDateTime;

use crate::{PartId, PartNote, PartNoteId, TbResult};

/// A trait representing a store for `PartNote` objects (part notes and file attachments).
///
/// These methods operate on metadata only; file bytes are stored separately and fetched on demand
/// via [`PartNoteStore::partnote_file`].
#[async_trait::async_trait]
pub trait PartNoteStore {
    /// Creates a free-text note on a part.
    async fn partnote_create_text(
        &mut self,
        part: PartId,
        name: String,
        created: OffsetDateTime,
    ) -> TbResult<PartNote>;

    /// Creates a file note on a part, storing the given bytes.
    #[allow(clippy::too_many_arguments)]
    async fn partnote_create_file(
        &mut self,
        part: PartId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        created: OffsetDateTime,
    ) -> TbResult<PartNote>;

    /// Retrieves all notes attached to a part.
    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>>;

    /// Retrieves a single note by ID.
    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote>;

    /// Retrieves the stored bytes of a file note.
    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>>;

    /// Updates the body of a text note.
    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote>;

    /// Updates the file content of a file note.
    async fn partnote_update_file(
        &mut self,
        id: PartNoteId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
    ) -> TbResult<PartNote>;

    /// Removes the file attachment from a file note, converting it to a text note.
    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote>;

    /// Deletes a note and returns its ID.
    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId>;
}
