use super::*;
use crate::event::{AspectType, Event, ObjectType};
use crate::test_support::{TestDomain, TestStravaSession, TestStravaStore, gear_json, strava_user};

fn setup() -> (TestStravaStore, TestStravaSession) {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let session = TestStravaSession::new(UserId::from(1), 42.into());
    (store, session)
}

#[tokio::test]
async fn upsert_creates_new_user() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    let user = StravaUser::upsert::<TestDomain>(
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
    let tb = store.user(&user.tendabike_id).unwrap();
    assert_eq!(tb.firstname, "First");
    assert_eq!(tb.name, "Last");
    assert!(
        store
            .stravauser_get_by_stravaid(&42.into())
            .await?
            .is_some()
    );
    Ok(())
}

#[tokio::test]
async fn upsert_updates_existing_user() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    TestDomain::create_user("Old".into(), "Name".into(), &None, &mut store).await?;
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let user = StravaUser::upsert::<TestDomain>(
        42.into(),
        "New",
        "Name",
        &None,
        Some(&RefreshToken::new("tok2".to_string())),
        &mut store,
    )
    .await?;
    assert_eq!(user.tendabike_id, UserId::from(1));
    let tb = store.user(&UserId::from(1)).unwrap();
    assert_eq!(tb.firstname, "New");
    let stored = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert_eq!(
        stored.refresh_token().map(|t| t.secret().to_string()),
        Some("tok2".to_string())
    );
    Ok(())
}

#[tokio::test]
async fn upsert_without_refresh_is_disabled() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    let user =
        StravaUser::upsert::<TestDomain>(43.into(), "First", "Last", &None, None, &mut store)
            .await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn read_stravauser() {
    let (mut store, _) = setup();
    let user = StravaUser::read::<TestDomain>(UserId::from(1), &mut store)
        .await
        .unwrap();
    assert_eq!(user.strava_id(), 42.into());
    assert_eq!(user.tb_id(), UserId::from(1));
    let res = StravaUser::read::<TestDomain>(UserId::from(2), &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn strava_id_read_option() {
    let (mut store, _) = setup();
    assert!(
        StravaId::read::<TestDomain>(&42.into(), &mut store)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        StravaId::read::<TestDomain>(&99.into(), &mut store)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn update_token() -> TbResult<()> {
    let (mut store, _) = setup();
    let user = StravaId::from(42)
        .update_token::<TestDomain>(Some(&"new".to_string()), &mut store)
        .await?;
    assert_eq!(
        user.refresh_token().map(|t| t.secret().to_string()),
        Some("new".to_string())
    );
    let user = StravaId::from(42)
        .update_token::<TestDomain>(None, &mut store)
        .await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn disable_removes_events_and_token() -> TbResult<()> {
    let (mut store, _) = setup();
    store
        .stravaevent_store(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    assert_eq!(store.event_count(), 1);
    StravaId::from(42).disable::<TestDomain>(&mut store).await?;
    assert_eq!(store.event_count(), 0);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn deauthorizes_user_and_events() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .stravaevent_store(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    user_deauthorize(&mut session, &mut store).await?;
    assert_eq!(session.deauthorizes, vec![42.into()]);
    assert_eq!(store.event_count(), 0);
    let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
    assert!(user.disabled());
    Ok(())
}

#[tokio::test]
async fn user_delete_removes_everything() -> TbResult<()> {
    let (mut store, mut session) = setup();
    store
        .stravaevent_store(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    user_delete(&mut session, &mut store).await?;
    assert_eq!(session.deauthorizes, vec![42.into()]);
    assert!(store.strava_users().is_empty());
    assert_eq!(store.event_count(), 0);
    assert!(store.user(&UserId::from(1)).is_none());
    Ok(())
}

#[tokio::test]
async fn stats_include_event_counts() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    TestDomain::create_user("A".into(), "User".into(), &None, &mut store).await?;
    TestDomain::create_user("B".into(), "User".into(), &None, &mut store).await?;
    store.insert_user(strava_user(UserId::from(1), 42, true));
    store.insert_user(strava_user(UserId::from(2), 43, false));
    store
        .stravaevent_store(Event {
            owner_id: 42.into(),
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            ..Default::default()
        })
        .await?;
    let stats = get_all_stats::<TestDomain>(&mut store).await?;
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
    let url = strava_url::<TestDomain>(1, &mut store).await.unwrap();
    assert_eq!(url, "https://strava.com/athletes/42");
    let res = strava_url::<TestDomain>(99, &mut store).await;
    assert!(matches!(res, Err(Error::NotFound(_))));
}

#[tokio::test]
async fn update_gear_imports_bikes_and_shoes() -> TbResult<()> {
    let (mut store, mut session) = setup();
    session.queue(
        "/athlete",
        r#"{"bikes":[{"id":"b1"}],"shoes":[{"id":"g1"}]}"#,
    );
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    session.queue("/gear/g1", &gear_json("g1", None));
    let parts = StravaUser::update_gear(&mut session, &mut store).await?;
    assert_eq!(parts.len(), 2);
    let bike = store.part(&parts[0]).unwrap();
    assert_eq!(bike.what, 1.into());
    let shoes = store.part(&parts[1]).unwrap();
    assert_eq!(shoes.what, 301.into());
    Ok(())
}
