#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate thiserror;

extern crate chrono;
extern crate rocket;
#[macro_use] 
extern crate rocket_contrib;
extern crate reqwest;
#[macro_use] 
extern crate log;
extern crate diesel;
#[macro_use] 
extern crate serde_derive;
extern crate serde_json;
extern crate jsonwebtoken;

pub mod error;
pub use error::*;
use anyhow::Context;
use rocket::http::*;
use time::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use chrono::{
    Utc,
    DateTime,
};

pub fn parse_time (time: Option<String>) -> TbResult<Option<DateTime<Utc>>> {
    if let Some(time) = time {
        return Ok(Some(DateTime::parse_from_rfc3339(&time).context("could not parse time")?.with_timezone(&Utc)))
    }
    Ok(None)
}


pub mod token {
    use rocket::http::Cookies;
    use rocket::request::Request;
    use jsonwebtoken::{Header, encode, decode, Validation};
    
    const MY_SECRET: &[u8] = b"9bjh34g2jh5hgjg";

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct UserToken {
        iat: i64,
        exp: i64,
        id: i32
    }

    pub fn id(request: &Request) -> TbResult<i32> {
        let token = token(request)?;

        let token_data = decode::<UserToken>(&token, MY_SECRET, &Validation::default())?;
        Ok(token_data.claims.id)
    }

    pub fn id_unsafe(token: &str) -> TbResult<i32> {
        let token_data = decode::<UserToken>(token, MY_SECRET, &Validation {validate_exp:false, ..Default::default()})?;
        Ok(token_data.claims.id)
    }

    pub fn token(request: &Request) -> TbResult<String> {
        let mut headers = request.headers().get("Authorization"); 

        if let Some(header) = headers.next() {
            ensure!(headers.next() == None, "Multiple Authentication header");

            let authorization_words = header.split_whitespace().collect::<Vec<_>>();

            if authorization_words.len() != 2 {
                bail!("malformed token");
            }
            if authorization_words[0] != "Bearer" {
                bail!("No Bearer token");
            }
            return Ok(String::from(authorization_words[1]));
        }

        let cookies = request.guard::<Cookies>().unwrap();
        if let Some(cookie) = cookies.get("token") {
            return Ok(String::from(cookie.value()))
        }
        
        bail!("No token provided")
    }

    pub fn store (request: &Request, id: i32, iat: i64, exp: i64) -> String{

        let my_claims = UserToken {iat, exp, id};
        let jwt = encode(&Header::default(), &my_claims, MY_SECRET).expect("Could not encode jwt");
        let token = Cookie::build("token", jwt.clone())
                        .same_site(SameSite::Lax)
                        .max_age(Duration::days(90))
                        .finish();
        
        let mut cookie_store = request.guard::<Cookies>().expect("request cookies");
        cookie_store.add(token);
        jwt
    }
}