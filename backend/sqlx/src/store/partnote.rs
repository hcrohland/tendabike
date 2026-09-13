use sqlx::FromRow;
use time::OffsetDateTime;

use crate::{SqlxConn, into_domain, vec_into};
use tb_domain::{NoteKind, PartId, PartNote, PartNoteId, TbResult};

#[derive(Clone, Debug, PartialEq, FromRow)]
struct DbPartNote {
    id: i32,
    part: i32,
    kind: String,
    name: String,
    mime: Option<String>,
    filename: Option<String>,
    size: Option<i64>,
    created: OffsetDateTime,
}

impl From<DbPartNote> for PartNote {
    fn from(db: DbPartNote) -> Self {
        let DbPartNote {
            id,
            part,
            kind,
            name,
            mime,
            filename,
            size,
            created,
        } = db;
        Self {
            id: id.into(),
            part: part.into(),
            kind: match kind.as_str() {
                "file" => NoteKind::File,
                _ => NoteKind::Text,
            },
            name,
            mime,
            filename,
            size,
            created,
        }
    }
}

#[async_trait::async_trait]
impl<'c> tb_domain::PartNoteStore for SqlxConn<'c> {
    async fn partnote_create_text(
        &mut self,
        part: PartId,
        name: String,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            r#"INSERT INTO part_notes (part, kind, name, created)
               VALUES ($1, 'text', $2, $3)
               RETURNING id, part, kind, name, mime, filename, size, created"#,
            i32::from(part),
            name,
            created
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_create_file(
        &mut self,
        part: PartId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            r#"INSERT INTO part_notes (part, kind, name, mime, filename, size, data, created)
               VALUES ($1, 'file', $2, $3, $4, $5, $6, $7)
               RETURNING id, part, kind, name, mime, filename, size, created"#,
            i32::from(part),
            name,
            mime,
            filename,
            size,
            data,
            created
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>> {
        sqlx::query_as!(
            DbPartNote,
            "SELECT id, part, kind, name, mime, filename, size, created FROM part_notes WHERE part = $1 ORDER BY id",
            i32::from(part)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            "SELECT id, part, kind, name, mime, filename, size, created FROM part_notes WHERE id = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>> {
        let data: Option<Vec<u8>> =
            sqlx::query_scalar!("SELECT data FROM part_notes WHERE id = $1", i32::from(id))
                .fetch_one(&mut **self.inner())
                .await
                .map_err(into_domain)?;
        Ok(data.unwrap_or_default())
    }

    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            "UPDATE part_notes SET name = $2 WHERE id = $1 RETURNING id, part, kind, name, mime, filename, size, created",
            i32::from(id),
            name
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_update_file(
        &mut self,
        id: PartNoteId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
    ) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            r#"UPDATE part_notes
               SET kind = 'file', name = $2, mime = $3, filename = $4, size = $5, data = $6
               WHERE id = $1
               RETURNING id, part, kind, name, mime, filename, size, created"#,
            i32::from(id),
            name,
            mime,
            filename,
            size,
            data
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        sqlx::query_as!(
            DbPartNote,
            r#"UPDATE part_notes
               SET kind = 'text', mime = NULL, filename = NULL, size = NULL, data = NULL
               WHERE id = $1
               RETURNING id, part, kind, name, mime, filename, size, created"#,
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId> {
        sqlx::query!("DELETE FROM part_notes WHERE id = $1", i32::from(id))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    fn db_note(kind: &str) -> DbPartNote {
        DbPartNote {
            id: 7,
            part: 3,
            kind: kind.to_string(),
            name: "torn tire".to_string(),
            mime: Some("image/png".to_string()),
            filename: Some("tire.png".to_string()),
            size: Some(1234),
            created: date(),
        }
    }

    #[test]
    fn from_db_part_note_maps_file_kind() {
        let note = PartNote::from(db_note("file"));
        assert_eq!(note.id, PartNoteId::from(7));
        assert_eq!(note.part, PartId::from(3));
        assert_eq!(note.kind, NoteKind::File);
        assert_eq!(note.name, "torn tire");
        assert_eq!(note.mime, Some("image/png".to_string()));
        assert_eq!(note.filename, Some("tire.png".to_string()));
        assert_eq!(note.size, Some(1234));
        assert_eq!(note.created, date());
    }

    #[test]
    fn from_db_part_note_maps_text_kind() {
        let db = DbPartNote {
            mime: None,
            filename: None,
            size: None,
            ..db_note("text")
        };
        let note = PartNote::from(db);
        assert_eq!(note.kind, NoteKind::Text);
        assert_eq!(note.mime, None);
        assert_eq!(note.filename, None);
        assert_eq!(note.size, None);
    }

    #[test]
    fn from_db_part_note_unknown_kind_falls_back_to_text() {
        let note = PartNote::from(db_note("garbage"));
        assert_eq!(note.kind, NoteKind::Text);
    }
}
