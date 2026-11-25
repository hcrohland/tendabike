use std::sync::{Arc, Mutex};

use super::super::event::{AspectType, Event, ObjectType};
use super::super::test_support::{gear_json, setup, strava_user};
use super::*;
use crate::facade::PartId;
use crate::store::Store;
use crate::strava::client::StravaAthleteInfo;
use crate::test_support::queue;

/// The athlete id round-trips through the `i64` conversions: a value
/// above `i32::MAX` is held intact.
#[test]
fn strava_id_round_trips_a_64bit_value() {
    let id: i64 = 5_000_000_000;
    assert_eq!(i64::from(StravaId::from(id)), id);
}

#[tokio::test]
async fn upsert_creates_new_user() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    // A new Strava user: the strava read is `None`, so the domain user is
    // created (via the facade) and the strava row is created.
    let created_user = Arc::new(Mutex::new(None));
    let created_user_rec = created_user.clone();
    let created_strava = Arc::new(Mutex::new(None));
    let created_strava_rec = created_strava.clone();
    store
        .expect_user_get_by_provider_id()
        .returning(|_id| Ok(None));
    facade
        .expect_create_user()
        .returning(move |firstname, lastname, _avatar, _store| {
            created_user_rec
                .lock()
                .unwrap()
                .replace((firstname.to_string(), lastname.to_string()));
            Ok(UserId::from(1))
        });
    store.expect_user_create().returning(move |su| {
        created_strava_rec.lock().unwrap().replace(su.clone());
        Ok(su)
    });
    let user = StravaUser::upsert(
        42.into(),
        "First",
        "Last",
        &None,
        Some(&RefreshToken::new("tok".to_string())),
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(user.id, 42.into());
    assert!(!user.disabled());
    let (firstname, lastname) = created_user
        .lock()
        .unwrap()
        .take()
        .expect("the domain user was created");
    assert_eq!(firstname, "First");
    assert_eq!(lastname, "Last");
    let su = created_strava
        .lock()
        .unwrap()
        .take()
        .expect("the strava row was created");
    assert_eq!(su.tendabike_id, UserId::from(1));
    assert_eq!(su.id, 42.into());
    Ok(())
}

/// The login's wire parse accepts a 64-bit athlete id: the token response's
/// `athlete` field deserializes an id above `i32::MAX`, and the upsert
/// stores the full value.
#[tokio::test]
async fn upsert_accepts_a_64bit_strava_id() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    let id: i64 = 5_000_000_000;
    // The token response's athlete extra fields, as the login parses them.
    let athlete: StravaAthleteInfo = serde_json::from_value(serde_json::json!({
        "id": id,
        "firstname": "First",
        "lastname": "Last",
        "profile_medium": null
    }))
    .unwrap();
    // A new Strava user: the strava read is `None`, so the domain user and
    // the strava row are created.
    store
        .expect_user_get_by_provider_id()
        .returning(|_id| Ok(None));
    facade
        .expect_create_user()
        .returning(|_f, _l, _a, _s| Ok(UserId::from(1)));
    let created_strava = Arc::new(Mutex::new(None));
    let created_strava_rec = created_strava.clone();
    store.expect_user_create().returning(move |su| {
        created_strava_rec.lock().unwrap().replace(su.clone());
        Ok(su)
    });
    let user = StravaUser::upsert(
        athlete.id,
        &athlete.firstname,
        &athlete.lastname,
        &athlete.avatar,
        Some(&RefreshToken::new("tok".to_string())),
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(user.id, StravaId::from(id));
    let su = created_strava
        .lock()
        .unwrap()
        .take()
        .expect("the strava row was created");
    assert_eq!(
        su.id,
        StravaId::from(id),
        "the row carries the full 64-bit id"
    );
    Ok(())
}

#[tokio::test]
async fn upsert_updates_existing_user() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    // An existing Strava user: the strava read returns the row (bound to tbid
    // 1), so the domain user is updated (via the facade) and the token is
    // refreshed.
    let updated_name: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_name_rec = updated_name.clone();
    let updated_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_token_rec = updated_token.clone();
    store
        .expect_user_get_by_provider_id()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    facade
        .expect_update_user()
        .returning(move |_uid, firstname, _lastname, _avatar, _store| {
            *updated_name_rec.lock().unwrap() = Some(firstname.to_string());
            Ok(UserId::from(1))
        });
    store
        .expect_user_update_token()
        .returning(move |_id, refresh| {
            *updated_token_rec.lock().unwrap() = refresh;
            Ok(strava_user(UserId::from(1), 42, true))
        });
    let user = StravaUser::upsert(
        42.into(),
        "New",
        "Name",
        &None,
        Some(&RefreshToken::new("tok2".to_string())),
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(user.tendabike_id, UserId::from(1));
    assert_eq!(
        updated_name.lock().unwrap().clone(),
        Some("New".to_string())
    );
    assert_eq!(
        updated_token.lock().unwrap().clone(),
        Some("tok2".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn upsert_without_refresh_is_disabled() -> TbResult<()> {
    let (mut store, mut facade, _) = setup();
    // No refresh token: the created strava row is disabled.
    store
        .expect_user_get_by_provider_id()
        .returning(|_id| Ok(None));
    facade
        .expect_create_user()
        .returning(|_f, _l, _a, _s| Ok(UserId::from(1)));
    store.expect_user_create().returning(Ok);
    let user =
        StravaUser::upsert(43.into(), "First", "Last", &None, None, &facade, &mut store).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn read_stravauser() {
    let (mut store, _facade, _) = setup();
    // `StravaUser::read` is a direct strava read: tbid 1 resolves, tbid 2 does
    // not.
    store.expect_user_get_by_tbid().returning(|id| {
        if id == UserId::from(1) {
            Ok(strava_user(UserId::from(1), 42, true))
        } else {
            Err(Error::NotFound(
                "user not registered with strava".to_string(),
            ))
        }
    });
    let user = StravaUser::read(UserId::from(1), &mut store).await.unwrap();
    assert_eq!(user.strava_id(), 42.into());
    assert_eq!(user.user_id(), UserId::from(1));
    let res = StravaUser::read(UserId::from(2), &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn strava_id_read_option() {
    let (mut store, _facade, _) = setup();
    // `StravaId::read` resolves a known strava id to `Some`, an unknown one to
    // `None`.
    store.expect_user_get_by_provider_id().returning(|id| {
        if *id == 42.into() {
            Ok(Some(strava_user(UserId::from(1), 42, true)))
        } else {
            Ok(None)
        }
    });
    assert!(
        StravaId::read(&42.into(), &mut store)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        StravaId::read(&99.into(), &mut store)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn update_token() -> TbResult<()> {
    let (mut store, _facade, _) = setup();
    // `update_token` is a direct strava write: the canned store returns the
    // user with the token it was given (or disabled for `None`).
    store
        .expect_user_update_token()
        .returning(move |_id, refresh| {
            Ok(StravaUser {
                id: 42.into(),
                tendabike_id: UserId::from(1),
                refresh_token: refresh.map(RefreshToken::new),
            })
        });
    let user = StravaId::from(42)
        .update_token(Some("new".to_string()), &mut store)
        .await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("new".to_string())
    );
    let user = StravaId::from(42).update_token(None, &mut store).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn disable_removes_events_and_token() -> TbResult<()> {
    let (mut store, _facade, _) = setup();
    // `disable` drops the user's events and clears their token. The canned
    // store records both side effects.
    store.expect_event_create().returning(|_e| Ok(()));
    let events_deleted = Arc::new(Mutex::new(0));
    let events_deleted_rec = events_deleted.clone();
    let token_refresh: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let token_refresh_rec = token_refresh.clone();
    // `user_disable` is the store's single disable op (drop the queued events,
    // clear the token): the canned store records both side effects.
    store.expect_user_disable().returning(move |_owner| {
        *events_deleted_rec.lock().unwrap() = 1;
        *token_refresh_rec.lock().unwrap() = None;
        Ok(())
    });
    store
        .event_create(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    StravaId::from(42).disable(&mut store).await?;
    assert_eq!(
        *events_deleted.lock().unwrap(),
        1,
        "the user's events are dropped"
    );
    assert_eq!(
        token_refresh.lock().unwrap().clone(),
        None,
        "the token is cleared (the user is disabled)"
    );
    Ok(())
}

#[tokio::test]
async fn user_strava_url() -> TbResult<()> {
    let (mut store, _facade, _) = setup();
    // `user_url` is a direct strava read: tbid 1 resolves to its athlete
    // URL, tbid 99 does not.
    store.expect_user_get_by_tbid().returning(|id| {
        if id == UserId::from(1) {
            Ok(strava_user(UserId::from(1), 42, true))
        } else {
            Err(Error::NotFound(
                "user not registered with strava".to_string(),
            ))
        }
    });
    use crate::driver::Provider;
    let url = super::super::driver::StravaDriver::user_url(UserId::from(1), &mut store).await?;
    assert_eq!(url, "https://strava.com/athletes/42");
    let res = super::super::driver::StravaDriver::user_url(UserId::from(99), &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
    Ok(())
}

#[tokio::test]
async fn update_gear_imports_bikes_and_shoes() -> TbResult<()> {
    let (mut store, mut facade, _session) = setup();
    queue(
        "/athlete",
        r#"{"bikes":[{"id":"b1"}],"shoes":[{"id":"g1"}]}"#,
    );
    queue("/gear/b1", &gear_json("b1", Some(0)));
    queue("/gear/g1", &gear_json("g1", None));
    // No gear is imported yet: each fetch creates a part. The facade
    // assigns ids in order and records each part's `what` (bike vs shoes).
    let next_id = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let next_id_rec = next_id.clone();
    let whats = Arc::new(Mutex::new(Vec::new()));
    let whats_rec = whats.clone();
    facade
        .expect_part_by_source()
        .returning(|_s, _store| Ok(None));
    facade.expect_create_part().returning(
        move |_name, _vendor, _model, what, _source, _purchase, _user, _store| {
            whats_rec.lock().unwrap().push(what);
            let id = next_id_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            Ok(super::super::test_support::part(
                PartId::from(id),
                UserId::from(1),
                what,
                None,
            ))
        },
    );
    use crate::driver::Provider;
    let parts =
        super::super::driver::StravaDriver::import_gear(UserId::from(1), &facade, &mut store)
            .await?;
    assert_eq!(parts, vec![PartId::from(1), PartId::from(2)]);
    assert_eq!(
        whats.lock().unwrap().as_slice(),
        [1.into(), 301.into()],
        "the bike is what 1, the shoes are what 301"
    );
    Ok(())
}
