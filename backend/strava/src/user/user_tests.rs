use std::sync::{Arc, Mutex};

use super::*;
use crate::event::{AspectType, Event, ObjectType};
use crate::test_support::{
    MockStravaConn, deauthorizes, gear_json, queue, strava_admin_session, strava_session,
    strava_user, user,
};
use tb_domain::MockSession;

fn setup() -> (MockStravaConn, MockSession) {
    let store = MockStravaConn::new();
    let session = strava_session(UserId::from(1), 42.into());
    (store, session)
}

#[tokio::test]
async fn upsert_creates_new_user() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    // A new Strava user: the strava read is `None`, so the domain user is
    // created and the strava row is created. The canned store records the
    // create's arguments (the names) and the strava row (the tbid binding).
    let created_user = Arc::new(Mutex::new(None));
    let created_user_rec = created_user.clone();
    let created_strava = Arc::new(Mutex::new(None));
    let created_strava_rec = created_strava.clone();
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(None));
    store
        .expect_user_create()
        .returning(move |firstname, lastname, _avatar| {
            created_user_rec
                .lock()
                .unwrap()
                .replace((firstname.to_string(), lastname.to_string()));
            Ok(user(UserId::from(1)))
        });
    store.expect_stravauser_create().returning(move |su| {
        created_strava_rec.lock().unwrap().replace(su.clone());
        Ok(su)
    });
    let user = StravaUser::upsert(
        42.into(),
        "First",
        "Last",
        &None,
        Some(&RefreshToken::new("tok".to_string())),
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

#[tokio::test]
async fn upsert_updates_existing_user() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    // An existing Strava user: the strava read returns the row (bound to tbid
    // 1), so the domain user is updated (not created) and the token is
    // refreshed. The canned store records the update's name and the new token.
    let updated_name: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_name_rec = updated_name.clone();
    let updated_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_token_rec = updated_token.clone();
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(Some(strava_user(UserId::from(1), 42, true))));
    store
        .expect_user_update()
        .returning(move |_uid, firstname, _lastname, _avatar| {
            *updated_name_rec.lock().unwrap() = Some(firstname.to_string());
            Ok(user(UserId::from(1)))
        });
    store
        .expect_stravauser_update_token()
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
    let mut store = MockStravaConn::new();
    // No refresh token: the created strava row is disabled.
    store
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(None));
    store
        .expect_user_create()
        .returning(move |_f, _l, _a| Ok(user(UserId::from(1))));
    store.expect_stravauser_create().returning(Ok);
    let user = StravaUser::upsert(43.into(), "First", "Last", &None, None, &mut store).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn read_stravauser() {
    let (mut store, _) = setup();
    // `StravaUser::read` is a direct strava read: tbid 1 resolves, tbid 2 does
    // not.
    store.expect_stravauser_get_by_tbid().returning(|id| {
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
    let (mut store, _) = setup();
    // `StravaId::read` resolves a known strava id to `Some`, an unknown one to
    // `None`.
    store.expect_stravauser_get_by_stravaid().returning(|id| {
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
    let (mut store, _) = setup();
    // `update_token` is a direct strava write: the canned store returns the
    // user with the token it was given (or disabled for `None`).
    store
        .expect_stravauser_update_token()
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
    let (mut store, _) = setup();
    // `disable` drops the user's events and clears their token. The canned
    // store records both side effects.
    store.expect_strava_event_create().returning(|_e| Ok(()));
    let events_deleted = Arc::new(Mutex::new(0));
    let events_deleted_rec = events_deleted.clone();
    let token_refresh: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_strava_events_delete_for_user()
        .returning(move |_user| {
            *events_deleted_rec.lock().unwrap() = 1;
            Ok(1)
        });
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            *token_refresh_rec.lock().unwrap() = refresh;
            Ok(strava_user(UserId::from(1), 42, false))
        });
    store
        .strava_event_create(Event {
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
async fn deauthorizes_user_and_events() -> TbResult<()> {
    let (mut store, _) = setup();
    // Deauthorize: the acting admin (user 2) revokes the target's (user 1)
    // Strava token (recorded in the thread-local seam) and drops their events
    // + token. The target's strava id (42) comes from the warm token cache
    // seeded by `setup`, so no strava read fires.
    store.expect_strava_event_create().returning(|_e| Ok(()));
    let events_deleted = Arc::new(Mutex::new(0));
    let events_deleted_rec = events_deleted.clone();
    let token_refresh: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_strava_events_delete_for_user()
        .returning(move |_user| {
            *events_deleted_rec.lock().unwrap() = 1;
            Ok(1)
        });
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            *token_refresh_rec.lock().unwrap() = refresh;
            Ok(strava_user(UserId::from(1), 42, false))
        });
    store
        .strava_event_create(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    // The acting admin is a distinct user from the target (tbid 1): the
    // target's token (strava 42) is revoked, not the actor's.
    let mut actor = strava_admin_session(UserId::from(2), 43.into());
    user_deauthorize(&mut actor, UserId::from(1), &mut store).await?;
    assert_eq!(deauthorizes(), vec![42.into()]);
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
async fn user_delete_removes_everything() -> TbResult<()> {
    let (mut store, _) = setup();
    // Delete: deauthorize (revokes the token, drops events) plus the strava
    // row and the domain cascade. The canned store records the strava row
    // delete, the token clear, and the domain user delete.
    store.expect_strava_event_create().returning(|_e| Ok(()));
    store
        .expect_strava_events_delete_for_user()
        .returning(|_user| Ok(1));
    let token_refresh: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let token_refresh_rec = token_refresh.clone();
    store
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            *token_refresh_rec.lock().unwrap() = refresh;
            Ok(strava_user(UserId::from(1), 42, false))
        });
    let strava_deleted = Arc::new(Mutex::new(0));
    let strava_deleted_rec = strava_deleted.clone();
    store.expect_stravauser_delete().returning(move |_user| {
        *strava_deleted_rec.lock().unwrap() = 1;
        Ok(1)
    });
    // The domain cascade reads the user's state (all empty) and deletes each
    // collection plus the user row.
    store.expect_get_all().returning(|_uid| Ok(Vec::new()));
    store
        .expect_shops_get_all_for_user()
        .returning(|_uid| Ok(Vec::new()));
    store
        .expect_part_get_all_for_userid()
        .returning(|_uid| Ok(Vec::new()));
    store.expect_by_user().returning(|_uid| Ok(Vec::new()));
    store
        .expect_notification_all_for_user()
        .returning(|_uid| Ok(Vec::new()));
    store.expect_services_delete().returning(|_s| Ok(0));
    store.expect_serviceplans_delete().returning(|_s| Ok(0));
    store
        .expect_attachments_delete_by_parts()
        .returning(|_p| Ok(0));
    store.expect_activities_delete().returning(|_a| Ok(0));
    store.expect_parts_delete().returning(|_p| Ok(0));
    store.expect_usages_delete().returning(|_u| Ok(0));
    let user_deleted = Arc::new(Mutex::new(0));
    let user_deleted_rec = user_deleted.clone();
    store.expect_user_delete().returning(move |_user| {
        *user_deleted_rec.lock().unwrap() = 1;
        Ok(1)
    });
    store
        .strava_event_create(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    // The acting admin is a distinct user from the target (tbid 1): the
    // target's token (strava 42) is revoked, not the actor's.
    let mut actor = strava_admin_session(UserId::from(2), 43.into());
    user_delete(&mut actor, UserId::from(1), &mut store).await?;
    assert_eq!(deauthorizes(), vec![42.into()]);
    assert_eq!(
        *strava_deleted.lock().unwrap(),
        1,
        "the strava row is deleted"
    );
    assert_eq!(
        token_refresh.lock().unwrap().clone(),
        None,
        "the token is cleared (the user is disabled)"
    );
    assert_eq!(
        *user_deleted.lock().unwrap(),
        1,
        "the domain user row is deleted"
    );
    Ok(())
}

#[tokio::test]
async fn deauthorize_requires_an_admin() {
    let (mut store, mut session) = setup();
    // A non-admin acting session is `Forbidden` (the domain enforces the
    // invariant, not just the web layer's AxumAdmin guard). No store read fires.
    let res = user_deauthorize(&mut session, UserId::from(1), &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn user_delete_requires_an_admin() {
    let (mut store, mut session) = setup();
    // A non-admin acting session is `Forbidden`. No store read fires.
    let res = user_delete(&mut session, UserId::from(1), &mut store).await;
    assert!(matches!(res, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn stats_include_event_counts() -> TbResult<()> {
    let mut store = MockStravaConn::new();
    // `get_all_stats` reads every strava user, each user's stat (user row,
    // parts, activities), and each user's event count.
    store.expect_stravauser_get_all().returning(|| {
        Ok(vec![
            strava_user(UserId::from(1), 42, true),
            strava_user(UserId::from(2), 43, false),
        ])
    });
    store.expect_user_get().returning(move |uid| Ok(user(uid)));
    store
        .expect_part_get_all_for_userid()
        .returning(|_uid| Ok(Vec::new()));
    store.expect_get_all().returning(|_uid| Ok(Vec::new()));
    store
        .expect_strava_events_get_count_for_user()
        .returning(move |user| Ok(if *user == 42.into() { 1 } else { 0 }));
    let stats = get_all_stats(&mut store).await?;
    assert_eq!(stats.len(), 2);
    let enabled = stats.iter().find(|s| !s.disabled).unwrap();
    assert_eq!(enabled.events, 1);
    let disabled = stats.iter().find(|s| s.disabled).unwrap();
    assert_eq!(disabled.events, 0);
    Ok(())
}

#[tokio::test]
async fn user_strava_url() {
    let (mut store, _) = setup();
    // `strava_url` is a direct strava read: tbid 1 resolves to its athlete
    // URL, tbid 99 does not.
    store.expect_stravauser_get_by_tbid().returning(|id| {
        if id == UserId::from(1) {
            Ok(strava_user(UserId::from(1), 42, true))
        } else {
            Err(Error::NotFound(
                "user not registered with strava".to_string(),
            ))
        }
    });
    let url = strava_url(UserId::from(1), &mut store).await.unwrap();
    assert_eq!(url, "https://strava.com/athletes/42");
    let res = strava_url(UserId::from(99), &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn update_gear_imports_bikes_and_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    queue(
        "/athlete",
        r#"{"bikes":[{"id":"b1"}],"shoes":[{"id":"g1"}]}"#,
    );
    queue("/gear/b1", &gear_json("b1", Some(0)));
    queue("/gear/g1", &gear_json("g1", None));
    // No gear is imported yet: each fetch creates a part. The canned store
    // assigns ids in order and records each part's `what` (bike vs shoes).
    let next_id = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let next_id_rec = next_id.clone();
    let whats = Arc::new(Mutex::new(Vec::new()));
    let whats_rec = whats.clone();
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    store
        .expect_part_create()
        .returning(move |what, _n, _v, _m, _p, _src, _u, _o, _s| {
            whats_rec.lock().unwrap().push(what);
            let id = next_id_rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            Ok(crate::test_support::part(
                PartId::from(id),
                UserId::from(1),
                what,
                None,
            ))
        });
    let parts = StravaUser::update_gear(&mut session, &mut store).await?;
    assert_eq!(parts, vec![PartId::from(1), PartId::from(2)]);
    assert_eq!(
        whats.lock().unwrap().as_slice(),
        [1.into(), 301.into()],
        "the bike is what 1, the shoes are what 301"
    );
    Ok(())
}
