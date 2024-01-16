use crate::{TbResult, User, UserId};

pub trait UserDomain {}
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
    async fn user_read_by_id(&mut self, uid: UserId) -> TbResult<User>;

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
    async fn user_create(&mut self, firstname: &str, lastname: &str) -> TbResult<User>;

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
    async fn user_update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
    ) -> TbResult<User>;
}
