
use super::error::*;
use rocket::http::*;
use time::*;


use rocket::http::Cookies;
use rocket::request::Request;
use jsonwebtoken::{Header, encode, decode, Validation};

pub const LEEWAY: i64 = 60 * 60 * 24 * 21; // 21 days

const MY_SECRET: &[u8] = b"9bjh34g2jh5hgjg";

use super::*;

#[derive(Debug, Serialize, Deserialize)]
struct UserToken {
    iat: i64,
    exp: i64,
    id: i32
}

pub fn id(token: &str, leeway: i64) -> TbResult<i32> {
    let token_data = decode::<UserToken>(token, MY_SECRET, &Validation {leeway, ..Default::default()})?;
    Ok(token_data.claims.id)
}

#[cfg(debug_assertions)]
const TOKEN: &str = "tendabike_debug";
#[cfg(not(debug_assertions))]
const TOKEN: &str = "tendabike_token";

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
                    .max_age(Duration::seconds(LEEWAY))
                    .finish()
}

pub fn store (cookie_store: &mut Cookies, id: i32, iat: i64, exp: i64) -> String{

    let my_claims = UserToken {iat, exp, id};
    let jwt = encode(&Header::default(), &my_claims, MY_SECRET).expect("Could not encode jwt");
    let token = cookie(jwt.clone());
    
    cookie_store.add(token);
    jwt
}

pub fn remove (mut cookie_store: Cookies) {
    cookie_store.remove(cookie(""))
}