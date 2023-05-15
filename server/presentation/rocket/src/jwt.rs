use anyhow::{ensure, bail};
use time::*;

use rocket::http::{Cookie, Cookies, SameSite};
use rocket::request::Request;
use jsonwebtoken::{DecodingKey, decode, Validation, Algorithm, encode, EncodingKey, Header};
use serde_derive::{Deserialize, Serialize};

pub const LEEWAY: u64 = 60 * 60 * 24 * 21; // 21 days

const MY_SECRET: &[u8] = b"9bjh34g2jh5hgjg";

use super::*;

#[derive(Debug, Serialize, Deserialize)]
struct UserToken {
    iat: i64,
    exp: i64,
    id: i32
}

pub fn id(token: &str) -> AnyResult<i32> {

    let token_data = decode::<UserToken>(token, &DecodingKey::from_secret(MY_SECRET), &Validation::new(Algorithm::HS256))?;
    Ok(token_data.claims.id)
}

#[cfg(debug_assertions)]
const TOKEN: &str = "tendabike_debug";
#[cfg(not(debug_assertions))]
const TOKEN: &str = "tendabike_token";

pub fn token(request: &Request) -> AnyResult<String> {
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
    if let Some(cookie) = cookies.get(TOKEN) {
        return Ok(String::from(cookie.value()))
    }
    
    bail!("No token provided")
}

fn cookie<T> (value: T) -> Cookie<'static>
    where T: Into<std::borrow::Cow<'static, str>>
{
    Cookie::build(TOKEN, value)
                    .same_site(SameSite::Strict)
                    .path("/")
                    .max_age(Duration::seconds(LEEWAY as i64))
                    .finish()
}

pub fn store (cookie_store: &mut Cookies, id: i32, exp: i64) {

    let iat = get_time().sec;
    let my_claims = UserToken {iat, exp, id};
    let jwt = encode(&Header::default(), &my_claims, &EncodingKey::from_secret(MY_SECRET)).expect("Could not encode jwt");
    
    cookie_store.add(cookie(jwt));
}

pub fn remove (mut cookie_store: Cookies) {
    cookie_store.remove(cookie(""))
}