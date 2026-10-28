//! The `PartNote` entity tests over a canned [`MockStore`] (issue #642).
//!
//! Each test runs the real part-note domain logic — the blank-name guard that
//! returns before any store call, the `!has_file()` guards, the
//! filename-fallback resolution in `update_file`, and the full permission
//! matrix (owner / foreign / shop-session) — over a [`MockStore`] whose
//! methods return canned rows. The domain logic runs for real; only the store
//! I/O is canned. The raw `partnote_*` adapter roundtrips the sqlx seam lacks
//! (list, `update_text`, `update_file`, `remove_file`) live in the sqlx seam
//! suite (`sqlx/tests/store_seam.rs`), where the Postgres behavior is the
//! source of truth; the read-after-write persistence pass-throughs are
//! covered there too, so they are dropped here.

use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::fixtures::sample_purchase_date;
use crate::test_support::{MockStore, mock_session, mock_session_with_shop};

fn created() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// A canned `PartNote` row: the minimal shape the domain logic needs.
fn note_row(
    id: PartNoteId,
    part: PartId,
    name: String,
    mime: Option<String>,
    filename: Option<String>,
    size: Option<i64>,
) -> PartNote {
    PartNote {
        id,
        part,
        name,
        mime,
        filename,
        size,
        created: created(),
    }
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner).
fn part_row(id: PartId, owner: UserId) -> Part {
    Part {
        id,
        owner,
        what: PartTypeId::from(1),
        name: "Main Bike".to_string(),
        vendor: "TendaBike".to_string(),
        model: "Standard".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    }
}

/// A canned `Shop` row: the minimal shape the domain logic needs (id + owner).
fn shop_row(id: ShopId, owner: UserId) -> Shop {
    Shop {
        id,
        owner,
        name: "Bike Barn".to_string(),
        description: None,
        auto_approve: true,
        created_at: sample_purchase_date(),
    }
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

// === Blank-name validation (returns before any store call) ===

/// note_create_text rejects a blank name before touching the store.
#[tokio::test]
async fn partid_note_create_text_rejects_blank_name() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = mock_session(UserId::from(1));
    let mut store = MockStore::new();
    let result = part
        .note_create_text(&owner, "   ".to_string(), created(), &mut store)
        .await;
    assert!(matches!(result, Err(Error::BadRequest(_))));
    Ok(())
}

/// update_text rejects a blank name before touching the store.
#[tokio::test]
async fn partnote_id_update_text_rejects_blank_name() -> TbResult<()> {
    let id = PartNoteId::from(1);
    let owner = mock_session(UserId::from(1));
    let mut store = MockStore::new();
    let result = id.update_text(&owner, "  ".to_string(), &mut store).await;
    assert!(matches!(result, Err(Error::BadRequest(_))));
    Ok(())
}

// === The `!has_file()` guards ===

/// remove_file on a text note is NotFound: the guard fires after the access
/// check, before the store's remove call.
#[tokio::test]
async fn partnote_id_remove_file_of_text_note_is_not_found() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let session = mock_session(owner);
    // A text note (no file) cannot be de-filed.
    let stored = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);

    let mut store = MockStore::new();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(stored.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));

    let result = PartNoteId::from(1).remove_file(&session, &mut store).await;
    assert!(matches!(result, Err(Error::NotFound(_))));
    Ok(())
}

/// file on a text note is NotFound: the guard fires before any store call.
#[tokio::test]
async fn partnote_file_of_text_note_is_not_found() -> TbResult<()> {
    let part = PartId::from(1);
    // A text note (no file) has no bytes to fetch; the guard returns before
    // any store call.
    let note = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);

    let mut store = MockStore::new();
    let result = note.file(&mut store).await;
    assert!(matches!(result, Err(Error::NotFound(_))));
    Ok(())
}

// === Filename-fallback resolution ===

/// update_file keeps the current filename and name when the multipart sends
/// neither: the domain resolves both before calling the store, and the
/// returned note echoes the resolved arguments.
#[tokio::test]
async fn partnote_id_update_file_falls_back_to_existing_filename() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let session = mock_session(owner);
    // The stored note already carries a filename; the update sends neither a
    // name nor a filename.
    let stored = note_row(
        PartNoteId::from(1),
        part,
        "a".to_string(),
        Some("image/png".to_string()),
        Some("photo.png".to_string()),
        Some(1),
    );

    let mut store = MockStore::new();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(stored.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    // The domain resolves the filename (and the name that falls back to it)
    // before calling the store; the returned note echoes those resolved args.
    store
        .expect_partnote_update_file()
        .returning(move |_id, name, mime, filename, size, _data| {
            Ok(note_row(
                PartNoteId::from(1),
                part,
                name,
                Some(mime),
                filename,
                Some(size),
            ))
        });

    let updated = PartNoteId::from(1)
        .update_file(
            &session,
            None,
            "image/png".to_string(),
            None,
            1,
            vec![2, 2],
            &mut store,
        )
        .await?;
    assert_eq!(updated.filename, Some("photo.png".to_string()));
    assert_eq!(updated.name, "photo.png");
    Ok(())
}

// === Permission matrix (owner / foreign / shop-session) ===

/// notes on an unknown part is NotFound: the access check reads the part first.
#[tokio::test]
async fn partid_notes_of_unknown_part_is_not_found() -> TbResult<()> {
    let part = PartId::from(99);
    let session = mock_session(UserId::from(1));
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(|_| Err(Error::NotFound("part".into())));
    let result = part.notes(&session, &mut store).await;
    assert!(matches!(result, Err(Error::NotFound(_))));
    Ok(())
}

/// notes of another user's part is Forbidden (no shop session).
#[tokio::test]
async fn partid_notes_of_foreign_part_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = part.notes(&stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// note for an unknown id is NotFound: the note read comes back empty.
#[tokio::test]
async fn partnote_id_note_of_unknown_id_is_not_found() -> TbResult<()> {
    let id = PartNoteId::from(99);
    let session = mock_session(UserId::from(1));
    let mut store = MockStore::new();
    store
        .expect_partnote_get()
        .returning(|_| Err(Error::NotFound("note".into())));
    let result = id.note(&session, &mut store).await;
    assert!(matches!(result, Err(Error::NotFound(_))));
    Ok(())
}

/// note of another user's part is Forbidden (no shop session).
#[tokio::test]
async fn partnote_id_note_of_foreign_part_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let note = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);
    let mut store = MockStore::new();
    let note_owned = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_owned.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = note.id.note(&stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// note_create_text of another user's part is Forbidden (no shop session).
#[tokio::test]
async fn partid_note_create_text_on_foreign_part_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = part
        .note_create_text(&stranger, "a".to_string(), created(), &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// note_create_file of another user's part is Forbidden (no shop session).
#[tokio::test]
async fn partid_note_create_file_on_foreign_part_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let (name, mime, filename, size, data) = file_args();
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = part
        .note_create_file(
            &stranger,
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

/// update_text of another user's note is Forbidden (no shop session).
#[tokio::test]
async fn partnote_id_update_text_of_foreign_note_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let note = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);
    let mut store = MockStore::new();
    let note_owned = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_owned.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = note
        .id
        .update_text(&stranger, "b".to_string(), &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// update_file of another user's note is Forbidden (no shop session).
#[tokio::test]
async fn partnote_id_update_file_of_foreign_note_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let note = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);
    let (name, mime, filename, size, data) = file_args();
    let mut store = MockStore::new();
    let note_owned = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_owned.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = note
        .id
        .update_file(
            &stranger,
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

/// remove_file of another user's note is Forbidden (no shop session).
#[tokio::test]
async fn partnote_id_remove_file_of_foreign_note_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let note = note_row(
        PartNoteId::from(1),
        part,
        "a".to_string(),
        Some("image/png".to_string()),
        Some("photo.png".to_string()),
        Some(1),
    );
    let mut store = MockStore::new();
    let note_owned = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_owned.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = note.id.remove_file(&stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// delete of another user's note is Forbidden (no shop session).
#[tokio::test]
async fn partnote_id_delete_of_foreign_note_is_forbidden() -> TbResult<()> {
    let part = PartId::from(1);
    let owner = UserId::from(1);
    let stranger = mock_session(UserId::from(98));
    let note = note_row(PartNoteId::from(1), part, "a".to_string(), None, None, None);
    let mut store = MockStore::new();
    let note_owned = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_owned.clone()));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, owner)));
    let result = note.id.delete(&stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// a shop owner with a session on the registered part can run the note
/// operations (create, list, update, delete) — the shop session passes the
/// part's access check via the shop's ownership.
#[tokio::test]
async fn shop_owner_can_access_partnotes() -> TbResult<()> {
    let part = PartId::from(1);
    let customer = UserId::from(1);
    let shop_owner = UserId::from(2);
    let shop = ShopId::from(1);

    let shop_session = mock_session_with_shop(shop_owner, shop);
    let note = note_row(
        PartNoteId::from(1),
        part,
        "check chain".to_string(),
        None,
        None,
        None,
    );

    let mut store = MockStore::new();
    // The part is the customer's; the shop owner acts in the shop, so
    // checkuser passes via the shop's ownership.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, customer)));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, shop_owner)));
    // create: the returned note echoes the supplied name.
    store
        .expect_partnote_create_text()
        .returning(move |_, name, _| {
            Ok(note_row(PartNoteId::from(1), part, name, None, None, None))
        });
    // list: one note after the create, none after the delete.
    let notes_counter = Arc::new(Mutex::new(0));
    let nc = notes_counter.clone();
    let note_for_list = note.clone();
    store.expect_partnote_all_by_part().returning(move |_| {
        let n = *nc.lock().unwrap();
        *nc.lock().unwrap() += 1;
        if n == 0 {
            Ok(vec![note_for_list.clone()])
        } else {
            Ok(vec![])
        }
    });
    // update and delete both read the note first (note()).
    let note_for_read = note.clone();
    store
        .expect_partnote_get()
        .returning(move |_| Ok(note_for_read.clone()));
    // update: the returned note echoes the new name.
    store
        .expect_partnote_update_text()
        .returning(move |_, name| Ok(note_row(PartNoteId::from(1), part, name, None, None, None)));
    store
        .expect_partnote_delete()
        .returning(|_| Ok(PartNoteId::from(1)));

    // create
    let created = part
        .note_create_text(
            &shop_session,
            "check chain".to_string(),
            created(),
            &mut store,
        )
        .await?;
    assert_eq!(created.name, "check chain");

    // list
    let notes = part.notes(&shop_session, &mut store).await?;
    assert_eq!(notes.len(), 1);

    // update
    let renamed = created
        .id
        .update_text(&shop_session, "chain worn".to_string(), &mut store)
        .await?;
    assert_eq!(renamed.name, "chain worn");

    // delete
    assert_eq!(
        created.id.delete(&shop_session, &mut store).await?,
        created.id
    );
    let notes = part.notes(&shop_session, &mut store).await?;
    assert!(notes.is_empty());
    Ok(())
}
