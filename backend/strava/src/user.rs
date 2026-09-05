//! This module contains the implementation of the StravaUser struct and its methods.
//!
//! The StravaUser struct represents a user of the Strava API and contains information such as the user's
//! Strava ID, Tendabike ID, access token, and refresh token.
//!
//! The methods implemented for the StravaUser struct allow for reading and updating user data, as well as
//! checking the validity of the user's access token.

use derive_more::{Display, From, Into};
use oauth2::RefreshToken;
use serde::Deserialize;

use crate::*;

#[derive(
    Clone, Copy, Debug, Display, From, Into, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct StravaId(i32);

impl StravaId {
    pub async fn read(&self, store: &mut impl StravaStore) -> TbResult<Option<StravaUser>> {
        store.stravauser_get_by_stravaid(self).await
    }

    /// update the refresh token for the user
    ///
    /// sets a five minute buffer for the access token
    /// returns the updated user
    pub async fn update_token(
        self,
        refresh: Option<&String>,
        store: &mut impl StravaStore,
    ) -> TbResult<StravaUser> {
        store.stravaid_update_token(self, refresh).await
    }

    /// disable a user
    pub(crate) async fn disable(self, store: &mut impl StravaStore) -> TbResult<()> {
        info!("disabling user {self}");

        let events = store.strava_events_delete_for_user(&self).await?;

        if events > 0 {
            info!("deleted {} open events for strava user {}", events, self);
        }

        store.stravaid_update_token(self, None).await?;
        Ok(())
    }
}

/// Strava User data
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct StravaUser {
    /// the Strava user id
    pub id: StravaId,
    /// the corresponding tendabike user id
    pub tendabike_id: UserId,
    /// the refresh token to get a new access token from Strava
    pub refresh_token: Option<RefreshToken>,
}

impl StravaUser {
    /// Reads the StravaUser data for the given `id` from the database.
    ///
    /// # Arguments
    ///
    /// * `id` - A `UserId` representing the Tendabike user ID.
    /// * `store` - A mutable reference to the database connection.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the user is not registered.
    pub async fn read(id: UserId, store: &mut impl StravaStore) -> TbResult<Self> {
        store.stravauser_get_by_tbid(id).await
    }

    /// read the current user data for id
    /// get the tendabike id for this user
    pub fn tb_id(&self) -> UserId {
        self.tendabike_id
    }

    /// get the strava id for this user
    pub fn strava_id(&self) -> StravaId {
        self.id
    }

    pub fn refresh_token(&self) -> Option<RefreshToken> {
        self.refresh_token.clone()
    }

    pub(crate) fn disabled(&self) -> bool {
        self.refresh_token.is_none()
    }

    /// Upsert a Strava user by ID, updating their Tendabike user ID if they already exist, or creating a new user if they don't.
    ///
    /// # Arguments
    ///
    /// * `id` - A `StravaId` representing the ID of the Strava user to upsert.
    /// * `firstname` - A `&str` representing the first name of the Strava user.
    /// * `lastname` - A `&str` representing the last name of the Strava user.
    /// * `store` - A mutable reference to a `AppConn` representing the database connection.
    ///
    /// # Returns
    ///
    /// An `TbResult` containing a `StravaUser` representing the upserted user.
    pub async fn upsert(
        id: StravaId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        refresh: Option<&RefreshToken>,
        store: &mut impl StravaStore,
    ) -> TbResult<StravaUser> {
        debug!("got id {}: {} {}", id, firstname, lastname);

        let user = id.read(store).await?;
        if let Some(user) = user {
            user.tendabike_id
                .update(firstname, lastname, avatar, store)
                .await?;
            store
                .stravaid_update_token(user.id, refresh.map(RefreshToken::secret))
                .await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = crate::UserId::create(firstname, lastname, avatar, store).await?;

        let user = StravaUser {
            id,
            tendabike_id,
            refresh_token: refresh.cloned(),
        };
        info!("creating new user id {user:?}");

        let user = store.stravauser_new(user).await?;
        // Note: Automatic sync removed - user must manually trigger initial activity import
        Ok(user)
    }

    /// Get list of gear for user from Strava
    pub async fn update_gear(
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Vec<PartId>> {
        #[derive(Deserialize, Debug)]
        struct Gear {
            id: String,
        }

        #[derive(Deserialize, Debug)]
        struct Athlete {
            // firstname: String,
            // lastname: String,
            bikes: Vec<Gear>,
            shoes: Vec<Gear>,
        }

        let ath: Athlete = user.request_json("/athlete", store).await?;

        let mut parts = Vec::new();
        for gear in ath.bikes.into_iter().chain(ath.shoes) {
            parts.push(gear::into_partid(gear.id, user, store).await?);
        }

        Ok(parts)
    }

    pub async fn process(
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        event::process(user, store).await
    }
}

#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
    disabled: bool,
}

pub async fn get_all_stats(store: &mut impl StravaStore) -> TbResult<Vec<StravaStat>> {
    let users = store.stravausers_get_all().await?;

    let mut res = Vec::new();
    for u in users {
        let stat = u.tendabike_id.get_stat(store).await?;
        let events = store.strava_events_get_count_for_user(&u.id).await?;
        res.push(StravaStat {
            stat,
            events,
            disabled: u.disabled(),
        });
    }
    Ok(res)
}

/// disable a user
///
/// # Errors
///
/// This function will return an error if the user does not exist, is already disabled
/// or has open events and if strava or the database is not reachable.
pub async fn user_deauthorize(
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<()> {
    if let Err(err) = user.deauthorize(store).await {
        warn!("could not deauthorize user {}: {:#}", user.tb_id(), err)
    }

    warn!("User {} deauthorized", user.tb_id());

    user.strava_id().disable(store).await
}

/// Returns the Strava URL for a user with the given Strava ID.
///
/// # Arguments
///
/// * `strava_id` - An `i32` representing the Strava ID of the user.
/// * `store` - A mutable reference to a `AppConn` representing the database connection.
///
/// # Returns
///
/// An `TbResult` containing a `String` representing the Strava URL for the user.
pub async fn strava_url(strava_id: i32, store: &mut impl StravaStore) -> TbResult<String> {
    let user_id = store.stravaid_get_user_id(strava_id).await?;
    Ok(format!("https://strava.com/athletes/{}", user_id))
}

pub async fn user_delete(
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<()> {
    let tbuser = user.tb_id();
    debug!("Deauthorizing user");
    user_deauthorize(user, store).await?;
    let n = store.stravauser_delete(tbuser).await?;
    debug!("Deleted {n} strava user");
    tbuser.delete(store).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{AspectType, Event, ObjectType};
    use crate::test_support::{TestStravaSession, TestStravaStore, gear_json, strava_user};

    fn setup() -> (TestStravaStore, TestStravaSession) {
        let mut store = TestStravaStore::new();
        store.insert_user(strava_user(UserId::from(1), 42, true));
        let session = TestStravaSession::new(UserId::from(1), 42.into());
        (store, session)
    }

    #[tokio::test]
    async fn upsert_creates_new_user() -> TbResult<()> {
        let mut store = TestStravaStore::new();
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
        let tb = UserStore::get(&mut store.mem, user.tendabike_id).await?;
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
        UserStore::create(&mut store.mem, "Old", "Name", &None).await?;
        store.insert_user(strava_user(UserId::from(1), 42, true));
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
        let tb = UserStore::get(&mut store.mem, UserId::from(1)).await?;
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
        let user = StravaUser::upsert(43.into(), "First", "Last", &None, None, &mut store).await?;
        assert!(user.disabled());
        Ok(())
    }

    #[tokio::test]
    async fn read_stravauser() {
        let (mut store, _) = setup();
        let user = StravaUser::read(UserId::from(1), &mut store).await.unwrap();
        assert_eq!(user.strava_id(), 42.into());
        assert_eq!(user.tb_id(), UserId::from(1));
        let res = StravaUser::read(UserId::from(2), &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn strava_id_read_option() {
        let (mut store, _) = setup();
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
        let user = StravaId::from(42)
            .update_token(Some(&"new".to_string()), &mut store)
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
        StravaId::from(42).disable(&mut store).await?;
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
        assert!(store.strava_users.is_empty());
        assert_eq!(store.event_count(), 0);
        let res = UserStore::get(&mut store.mem, UserId::from(1)).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
        Ok(())
    }

    #[tokio::test]
    async fn stats_include_event_counts() -> TbResult<()> {
        let mut store = TestStravaStore::new();
        UserStore::create(&mut store.mem, "A", "User", &None).await?;
        UserStore::create(&mut store.mem, "B", "User", &None).await?;
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
        let url = strava_url(1, &mut store).await.unwrap();
        assert_eq!(url, "https://strava.com/athletes/42");
        let res = strava_url(99, &mut store).await;
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
        let bike = PartStore::partid_get_part(&mut store.mem, parts[0]).await?;
        assert_eq!(bike.what, 1.into());
        let shoes = PartStore::partid_get_part(&mut store.mem, parts[1]).await?;
        assert_eq!(shoes.what, 301.into());
        Ok(())
    }
}
