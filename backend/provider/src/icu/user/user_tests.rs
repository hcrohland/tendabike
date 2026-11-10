use std::sync::{Arc, Mutex};

use super::super::test_support::{icu_user, setup};
use super::*;

#[tokio::test]
async fn upsert_creates_new_user() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // A new icu user: the icu read is `None`, so the domain user is created
    // (via the facade, with `provider = 'icu'`) and the icu row is created.
    let created_user = Arc::new(Mutex::new(None));
    let created_user_rec = created_user.clone();
    let created_provider = Arc::new(Mutex::new(None));
    let created_provider_rec = created_provider.clone();
    let created_icu = Arc::new(Mutex::new(None));
    let created_icu_rec = created_icu.clone();
    store.expect_icuser_get_by_icuid().returning(|_id| Ok(None));
    facade
        .expect_create_user()
        .returning(move |firstname, lastname, _avatar, provider, _store| {
            created_user_rec
                .lock()
                .unwrap()
                .replace((firstname.to_string(), lastname.to_string()));
            created_provider_rec
                .lock()
                .unwrap()
                .replace(provider.to_string());
            Ok(UserId::from(1))
        });
    store.expect_icuser_create().returning(move |su| {
        created_icu_rec.lock().unwrap().replace(su.clone());
        Ok(su)
    });
    let user = IcuUser::upsert(
        "ath-42".into(),
        "First",
        "Last",
        &None,
        "durable-token",
        &facade,
        &mut store,
    )
    .await?;
    assert_eq!(user.id, "ath-42".into());
    assert_eq!(
        created_provider.lock().unwrap().clone(),
        Some("icu".to_string()),
        "a new icu user is created with provider 'icu' (spec #763)"
    );
    let (firstname, lastname) = created_user
        .lock()
        .unwrap()
        .take()
        .expect("the domain user was created");
    assert_eq!(firstname, "First");
    assert_eq!(lastname, "Last");
    let su = created_icu
        .lock()
        .unwrap()
        .take()
        .expect("the icu row was created");
    assert_eq!(su.tendabike_id, UserId::from(1));
    assert_eq!(su.id, "ath-42".into());
    assert_eq!(su.access_token.as_deref(), Some("durable-token"));
    Ok(())
}

#[tokio::test]
async fn upsert_updates_existing_user() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    // An existing icu user: the icu read returns the row (bound to tbid 1), so
    // the domain user is updated (via the facade) and the token is refreshed.
    let updated_name: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_name_rec = updated_name.clone();
    let updated_token: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let updated_token_rec = updated_token.clone();
    store
        .expect_icuser_get_by_icuid()
        .returning(|_id| Ok(Some(icu_user(UserId::from(1), "ath-42", true))));
    facade
        .expect_update_user()
        .returning(move |_uid, firstname, _lastname, _avatar, _store| {
            *updated_name_rec.lock().unwrap() = Some(firstname.to_string());
            Ok(UserId::from(1))
        });
    store
        .expect_icuser_update_token()
        .returning(move |_id, access_token| {
            *updated_token_rec.lock().unwrap() = access_token;
            Ok(icu_user(UserId::from(1), "ath-42", true))
        });
    let user = IcuUser::upsert(
        "ath-42".into(),
        "New",
        "Name",
        &None,
        "new-durable-token",
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
        Some("new-durable-token".to_string())
    );
    Ok(())
}
