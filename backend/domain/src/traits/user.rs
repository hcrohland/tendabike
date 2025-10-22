use crate::{TbResult, User, UserId};

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
}
