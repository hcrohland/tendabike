use super::*;

pub trait Person {
    fn get_id(&self) -> i32;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: i32, error: String) -> AnyResult<()> {
        if self.get_id() == owner || self.is_admin() {
            Ok(())
        } else {
            Err(Error::Forbidden(error).into())
        }
    }
}
