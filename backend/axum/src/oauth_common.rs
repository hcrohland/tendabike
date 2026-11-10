//! Shared OAuth CSRF state handling: the HMAC-signed state token used by both
//! the Strava and intervals.icu OAuth flows. The token encodes the return path
//! and a random nonce, signed with the `CSRF_SECRET` (or a random key in
//! development).

use log::warn;
use oauth2::CsrfToken;
use std::sync::LazyLock;

use tb_domain::{Error, TbResult};

static CSRF_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    if let Ok(secret) = std::env::var("CSRF_SECRET") {
        hex::decode(secret).expect("CSRF_SECRET must be valid hex")
    } else {
        warn!("generating random CSRF secret. This does not work for distributed systems");
        (0..16).map(|_| rand::random::<u8>()).collect()
    }
});

fn hmac_signature(key: &[u8], msg: &str) -> String {
    use base64::prelude::*;
    use hmac::{Hmac, KeyInit, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(msg.as_bytes());
    let signature = mac.finalize().into_bytes();

    BASE64_STANDARD.encode(signature)
}

/// Generate a CSRF state token encoding `path` (the OAuth return path).
pub fn gentoken(path: String) -> CsrfToken {
    use base64::prelude::*;

    let random: Vec<u8> = (0..16).map(|_| rand::random()).collect();
    let random = BASE64_URL_SAFE_NO_PAD.encode(random);
    let msg = path + "+" + &random;
    let sig = hmac_signature(&CSRF_KEY, &msg);

    CsrfToken::new(msg + ":" + &sig)
}

/// Extract and verify the return path from a CSRF state token.
pub fn getpath(state: String) -> TbResult<String> {
    let msg = state.split(':').collect::<Vec<_>>();
    if msg.len() != 2 || hmac_signature(&CSRF_KEY, msg[0]) != msg[1] {
        return Err(Error::BadRequest(format!(
            "Bad signature for exchange request: {state}"
        )));
    };
    Ok(msg[0].split('+').next().unwrap_or("").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_signature_matches_known_vector() {
        assert_eq!(
            hmac_signature(b"tendabike", "/+abc123"),
            "jNqHCYY2M852nJCKpclLqF36vPQPrFtbECZMBvgKKlI="
        );
    }

    #[test]
    fn getpath_recovers_path_from_gentoken() {
        let token = gentoken("/foo/bar".to_string());
        assert_eq!(
            getpath(token.secret().clone()).expect("valid state"),
            "/foo/bar"
        );
    }

    #[test]
    fn getpath_rejects_corrupted_signature() {
        let token = gentoken("/foo".to_string());
        let mut state = token.secret().clone();
        state.push('x');
        assert!(matches!(getpath(state), Err(Error::BadRequest(_))));
    }

    #[test]
    fn getpath_rejects_malformed_state() {
        assert!(matches!(
            getpath("no-colon-separator".to_string()),
            Err(Error::BadRequest(_))
        ));
    }
}
