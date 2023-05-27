use super::*;

pub trait Person {
    fn get_id(&self) -> UserId;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: UserId, error: String) -> AnyResult<()> {
        if self.get_id() == owner || self.is_admin() {
            Ok(())
        } else {
            Err(Error::Forbidden(error).into())
        }
    }
}
