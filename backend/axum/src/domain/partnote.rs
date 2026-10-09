//! This module contains the implementation of the part note API endpoints.
//!
//! Part notes allow users to attach text notes or files to parts. The endpoints support:
//!
//! - `GET /{part}/notes` — list all notes for a part
//! - `POST /{part}/notes` — create a text note
//! - `POST /{part}/notes/file` — upload a file note (multipart)
//! - `GET /notes/{id}/file` — download file content (404 for a text note)
//! - `PUT /notes/{id}/file` — update a file note's attachment; also converts a text note
//!   into a file note (multipart)
//! - `DELETE /notes/{id}/file` — remove a file note's attachment, converting it to a text
//!   note (404 for a text note)
//! - `PUT /notes/{id}` — update a text note
//! - `DELETE /notes/{id}` — delete a note
//!
//! The handlers are transport-only: each extracts the path and session, parses the multipart
//! body where present, and drives the domain operation between `begin()` and `commit()`.
//! Permission checks, validation, and state fallbacks live in the domain layer
//! (`tb_domain::{PartId, PartNote, PartNoteId}`).

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::Deserialize;
use tb_domain::{ApiWrite, Error, NoteFile, PartId, PartNote, PartNoteId};
use tb_exec::{Txn, TxnSource};
use tb_strava::{StravaSession, StravaStore};

use crate::{
    RequestSession,
    appstate::AppState,
    domain::created_entity,
    error::{ApiResult, AppError},
};

/// Body for creating a text note.
#[derive(Clone, Debug, Deserialize)]
pub struct NewTextNote {
    pub name: String,
}

/// Body for updating a text note.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateTextNote {
    pub name: String,
}

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
    Router::new()
        .route("/{part}/notes", get(list_notes).post(create_text_note))
        .route("/{part}/notes/file", post(create_file_note))
        .route(
            "/notes/{id}/file",
            get(get_note_file)
                .put(update_file_note)
                .delete(remove_file_note),
        )
        .route("/notes/{id}", put(update_text_note))
        .route("/notes/{id}", delete(delete_note))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
}

async fn list_notes<S>(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<PartNote>>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let notes = part.notes(&user, &mut store).await?;
    store.commit().await?;
    Ok(Json(notes))
}

async fn create_text_note<S>(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(NewTextNote { name }): Json<NewTextNote>,
) -> Result<(StatusCode, Json<PartNote>), AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let summary = state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartNoteCreateText { part, name },
        )
        .await?;
    let note = created_entity(&summary.part_notes, "part note")?;
    Ok((StatusCode::CREATED, Json(note)))
}

/// A parsed multipart note body: the free-text name (if present and non-blank), the mime
/// type, the uploaded filename (if present), and the file bytes.
struct ParsedNoteFile {
    name: Option<String>,
    mime: String,
    filename: Option<String>,
    data: Vec<u8>,
}

fn mp_err(e: impl std::fmt::Display) -> AppError {
    AppError::TbError(Error::BadRequest(e.to_string()))
}

/// Parses the multipart body: a required file field plus an optional `name` field.
async fn parse_note_file(mut multipart: Multipart) -> Result<ParsedNoteFile, AppError> {
    let mut note_name: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_content_type: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(mp_err)? {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "name" {
            let value = field.text().await.map_err(mp_err)?;
            if !value.trim().is_empty() {
                note_name = Some(value);
            }
        } else if field.file_name().is_some() {
            file_name = field.file_name().map(|s| s.to_string());
            file_content_type = field
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            file_data = Some(field.bytes().await.map_err(mp_err)?.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| mp_err("no file field in multipart body"))?;
    Ok(ParsedNoteFile {
        name: note_name,
        mime: file_content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
        filename: file_name,
        data,
    })
}

async fn create_file_note<S>(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<PartNote>), AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let parsed = parse_note_file(multipart).await?;
    let summary = state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartNoteCreateFile {
                part,
                file: NoteFile {
                    name: parsed.name,
                    mime: parsed.mime,
                    filename: parsed.filename,
                    data: parsed.data,
                },
            },
        )
        .await?;
    let note = created_entity(&summary.part_notes, "part note")?;
    Ok((StatusCode::CREATED, Json(note)))
}

async fn get_note_file<S>(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<Response, AppError>
where
    S: TxnSource + Clone + 'static,
{
    let mut store = state.source.begin().await?;
    let note = id.note(&user, &mut store).await?;
    let data = note.file(&mut store).await?;
    store.commit().await?;

    let mime = note
        .mime
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let filename = note.filename.clone().unwrap_or_else(|| note.name.clone());
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        mime.parse()
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    let encoded = rfc5987_encode(&filename);
    let ascii_fallback: String = filename
        .chars()
        .map(|c| if c.is_ascii() && c != '"' { c } else { '_' })
        .collect();
    let disposition_type = if is_inline_image(&mime) {
        "inline"
    } else {
        "attachment"
    };
    let disposition =
        format!("{disposition_type}; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}");
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        disposition
            .parse()
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    Ok((headers, data).into_response())
}

async fn update_text_note<S>(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
    Json(UpdateTextNote { name }): Json<UpdateTextNote>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartNoteUpdateText { id, name },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_file_note<S>(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
    multipart: Multipart,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let parsed = parse_note_file(multipart).await?;
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartNoteUpdateFile {
                id,
                file: NoteFile {
                    name: parsed.name,
                    mime: parsed.mime,
                    filename: parsed.filename,
                    data: parsed.data,
                },
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_note<S>(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(&state.source, user.tb_id(), ApiWrite::PartNoteDelete { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_file_note<S>(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> Result<StatusCode, AppError>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    state
        .registry
        .write(
            &state.source,
            user.tb_id(),
            ApiWrite::PartNoteRemoveFile { id },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

fn rfc5987_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &byte in s.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'$' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'_' | b'~'
            )
        {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn is_inline_image(mime: &str) -> bool {
    let media = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    matches!(
        media.as_str(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    )
}

#[cfg(test)]
mod tests {
    use super::is_inline_image;

    #[test]
    fn inline_for_safe_raster_images() {
        assert!(is_inline_image("image/png"));
        assert!(is_inline_image("image/jpeg"));
        assert!(is_inline_image("image/webp"));
        assert!(is_inline_image("image/gif"));
    }

    #[test]
    fn not_inline_for_svg_and_non_images() {
        assert!(!is_inline_image("image/svg+xml"));
        assert!(!is_inline_image("application/octet-stream"));
        assert!(!is_inline_image("text/html"));
    }

    #[test]
    fn robust_to_case_and_parameters() {
        assert!(!is_inline_image("Image/SVG+XML"));
        assert!(is_inline_image("image/png; charset=binary"));
        assert!(!is_inline_image("image/svg+xml; charset=utf-8"));
    }
}
