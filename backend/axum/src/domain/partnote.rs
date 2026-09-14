//! This module contains the implementation of the part note API endpoints.
//!
//! Part notes allow users to attach text notes or files to parts. The endpoints support:
//!
//! - `GET /{part}/notes` — list all notes for a part
//! - `POST /{part}/notes` — create a text note
//! - `POST /{part}/notes/file` — upload a file note (multipart)
//! - `GET /notes/{id}/file` — download file content
//! - `PUT /notes/{id}/file` — update a file note's attachment (multipart)
//! - `DELETE /notes/{id}/file` — remove a file note's attachment (converts to text)
//! - `PUT /notes/{id}` — update a text note
//! - `DELETE /notes/{id}` — delete a note

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use serde::Deserialize;
use tb_domain::{Error, PartId, PartNote, PartNoteId, PartNoteStore, Store};

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
    let size = data.len() as i64;
    let filename = file_name.unwrap_or_else(|| "file".to_string());
    let name = note_name.unwrap_or_else(|| filename.clone());
    let content_type = file_content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let note = store
        .partnote_create_file(
            part,
            name,
            content_type,
            Some(filename),
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
    if !note.has_file() {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note is not a file".to_string(),
        )));
    }
    let data = store.partnote_file(id).await?;
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
    let res = store.partnote_update_text(id, name).await?;
    store.commit().await?;
    Ok(Json(res))
}

async fn update_file_note(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(store): State<DbPool>,
    mut multipart: Multipart,
) -> Result<Json<PartNote>, crate::error::AppError> {
    let mut store = store.begin().await?;
    let note = store.partnote_get(id).await?;
    let _ = note.part.part(&user, &mut store).await?;
    if !note.has_file() {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note is not a file".to_string(),
        )));
    }

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
    let size = data.len() as i64;
    let filename = file_name.unwrap_or_else(|| note.filename.unwrap_or_else(|| "file".to_string()));
    let name = note_name.unwrap_or_else(|| filename.clone());
    let content_type = file_content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let res = store
        .partnote_update_file(id, name, content_type, Some(filename), size, data)
        .await?;
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

async fn remove_file_note(
    Path(id): Path<PartNoteId>,
    user: RequestSession,
    State(store): State<DbPool>,
) -> ApiResult<PartNote> {
    let mut store = store.begin().await?;
    let note = store.partnote_get(id).await?;
    let _ = note.part.part(&user, &mut store).await?;
    if !note.has_file() {
        return Err(crate::error::AppError::TbError(Error::BadRequest(
            "note is not a file".to_string(),
        )));
    }
    let res = store.partnote_remove_file(id).await?;
    store.commit().await?;
    Ok(Json(res))
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
