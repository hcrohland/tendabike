use crate::{OnboardingStatus, TbResult, User, UserId};

#[async_trait::async_trait]
/// A trait representing a user store.
pub trait UserStore {
    /// Reads a user by their ID.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to read.
    ///
    /// # Returns
    ///
    /// The user with the given ID, if it exists.
    async fn get(&mut self, uid: UserId) -> TbResult<User>;

    /// Creates a new user.
    ///
    /// # Arguments
    ///
    /// * `firstname_` - The first name of the user.
    /// * `lastname_` - The last name of the user.
    ///
    /// # Returns
    ///
    /// The newly created user.
    async fn create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User>;

    /// Updates an existing user.
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to update.
    /// * `firstname_` - The new first name of the user.
    /// * `lastname_` - The new last name of the user.
    ///
    /// # Returns
    ///
    /// The updated user.
    async fn update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User>;

    /// Deletes a user
    ///
    /// # Arguments
    ///
    /// * `user` - The user to delete
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing 1 or an error if the operation fails.
    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize>;

    /// Updates the onboarding status for a user
    ///
    /// # Arguments
    ///
    /// * `uid` - The ID of the user to update
    /// * `status` - The new onboarding status
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the updated user or an error if the operation fails.
    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User>;
}
