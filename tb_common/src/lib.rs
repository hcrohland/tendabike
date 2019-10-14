
#[macro_use] 
extern crate error_chain;
extern crate rocket;
#[macro_use] 
extern crate rocket_contrib;
#[macro_use]
extern crate log;

pub mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
