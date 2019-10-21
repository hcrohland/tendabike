
extern crate anyhow;
#[macro_use] 
extern crate thiserror;

extern crate rocket;
#[macro_use] 
extern crate rocket_contrib;
#[macro_use] 
extern crate log;
extern crate diesel;

pub mod error;
pub use error::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
