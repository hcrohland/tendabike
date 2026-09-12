//! This module contains the implementation of the part note API endpoints.
//!
//! Part notes allow users to attach text notes or files to parts. The endpoints support:
//!
//! - `GET /{part}/notes` — list all notes for a part
//! - `POST /{part}/notes` — create a text note
//! - `POST /{part}/notes/file` — upload a file note (multipart)
//! - `GET /notes/{id}/file` — download file content
//! - `PUT /notes/{id}` — update a text note
//! - `DELETE /notes/{id}` — delete a note

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;
use tb_domain::{Error, NoteKind, PartId, PartNote, PartNoteId, PartNoteStore, Store};

use crate::{DbPool, RequestSession, appstate::AppState, error::ApiResult};

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

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/{part}/notes", get(list_notes).post(create_text_note))
        .route("/{part}/notes/file", post(create_file_note))
        .route("/notes/{id}/file", get(get_note_file))
        .route("/notes/{id}", put(update_text_note))
        .route("/notes/{id}", axum::routing::delete(delete_note))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
}

async fn list_notes(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(store): State<DbPool>,
) -> ApiResult<Vec<PartNote>> {
    let mut store = store.begin().await?;
    let _ = part.part(&user, &mut store).await?;
    let notes = store.partnote_all_by_part(part).await?;
    store.commit().await?;
    Ok(Json(notes))
}

async fn create_text_note(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(store): State<DbPool>,
    Json(NewTextNote { name }): Json<NewTextNote>,
) -> Result<(StatusCode, Json<PartNote>), crate::error::AppError> {
    if name.trim().is_empty() {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note name must not be empty".to_string(),
        )));
    }
    let mut store = store.begin().await?;
    let _ = part.part(&user, &mut store).await?;
    let note = store
        .partnote_create_text(part, name, time::OffsetDateTime::now_utc())
        .await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(note)))
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

fn mp_err(e: impl std::fmt::Display) -> crate::error::AppError {
    crate::error::AppError::TbError(Error::BadRequest(e.to_string()))
}

async fn create_file_note(
    Path(part): Path<PartId>,
    user: RequestSession,
    State(store): State<DbPool>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<PartNote>), crate::error::AppError> {
    let mut store = store.begin().await?;
    let _ = part.part(&user, &mut store).await?;

    let field = multipart
        .next_field()
        .await
        .map_err(mp_err)?
        .ok_or_else(|| mp_err("no file field in multipart body"))?;

    let name = field.file_name().unwrap_or("file").to_string();
    let content_type = field
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let data = field.bytes().await.map_err(mp_err)?.to_vec();
    let size = data.len() as i64;

    let note = store
        .partnote_create_file(
            part,
            name,
            content_type,
            size,
            data,
            time::OffsetDateTime::now_utc(),
        )
        .await?;
    store.commit().await?;
    Ok((StatusCode::CREATED, Json(note)))
}

async fn get_note_file(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(store): State<DbPool>,
) -> Result<Response, crate::error::AppError> {
    let mut store = store.begin().await?;
    let note = store.partnote_get(id).await?;
    let _ = note.part.part(&user, &mut store).await?;
    if note.kind != NoteKind::File {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note is not a file".to_string(),
        )));
    }
    let data = store.partnote_file(id).await?;
    store.commit().await?;

    let mime = note
        .mime
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let filename = note.name.clone();
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        mime.parse()
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    let encoded = rfc5987_encode(&filename);
    let ascii_fallback: String = filename
        .chars()
        .map(|c| if c.is_ascii() && c != '"' { c } else { '_' })
        .collect();
    let disposition =
        format!("attachment; filename=\"{ascii_fallback}\"; filename*=UTF-8''{encoded}");
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        disposition
            .parse()
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    Ok((headers, data).into_response())
}

async fn update_text_note(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(store): State<DbPool>,
    Json(UpdateTextNote { name }): Json<UpdateTextNote>,
) -> ApiResult<PartNote> {
    if name.trim().is_empty() {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note name must not be empty".to_string(),
        )));
    }
    let mut store = store.begin().await?;
    let note = store.partnote_get(id).await?;
    let _ = note.part.part(&user, &mut store).await?;
    if note.kind != NoteKind::Text {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note is not a text note".to_string(),
        )));
    }
    let res = store.partnote_update_text(id, name).await?;
    store.commit().await?;
    Ok(Json(res))
}

async fn delete_note(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(store): State<DbPool>,
) -> ApiResult<PartNoteId> {
    let mut store = store.begin().await?;
    let note = store.partnote_get(id).await?;
    let _ = note.part.part(&user, &mut store).await?;
    let res = store.partnote_delete(id).await?;
    store.commit().await?;
    Ok(Json(res))
}
