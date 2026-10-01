use super::*;
use crate::{Error, OnboardingStatus, TbResult, User, UserId};

#[async_trait::async_trait]
impl UserStore for MemStore {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        let d = self.state();
        d.users
            .get(&uid)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("User {} not found", uid)))
    }

    async fn create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        let d = self.state_mut();
        let id = UserId::from(d.next_user_id);
        d.next_user_id += 1;
        let user = User {
            id,
            firstname: firstname.into(),
            name: lastname.into(),
            avatar: avatar.clone(),
            is_admin: false,
            onboarding_status: OnboardingStatus::Pending,
        };
        d.users.insert(id, user.clone());
        Ok(user)
    }

    async fn update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        let d = self.state_mut();
        match d.users.get_mut(uid) {
            Some(user) => {
                user.firstname = firstname.into();
                user.name = lastname.into();
                user.avatar = avatar.clone();
                Ok(user.clone())
            }
            None => Err(Error::NotFound(format!("User {} not found", uid))),
        }
    }

    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        let d = self.state_mut();
        if d.users.remove(user).is_some() {
            Ok(1)
        } else {
            Ok(0)
        }
    }

    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User> {
        let d = self.state_mut();
        match d.users.get_mut(uid) {
            Some(user) => {
                user.onboarding_status = status;
                Ok(user.clone())
            }
            None => Err(Error::NotFound(format!("User {} not found", uid))),
        }
    }
}
