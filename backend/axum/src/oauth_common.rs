//! Shared OAuth handling for both the Strava and intervals.icu flows: the
//! HMAC-signed CSRF state token (the token encodes the return path and a
//! random nonce, signed with the `CSRF_SECRET` (or a random key in
//! development); the intervals.icu variant additionally encodes the PKCE code
//! verifier) and the shared callback completion (the provider-agnostic
//! `login` direct op, the tower-session write, and the redirect).

use axum::response::{IntoResponse, Redirect};
use log::{debug, error, warn};
use oauth2::CsrfToken;
use std::sync::OnceLock;
use tb_domain::{Error, TbResult};
use tb_provider::Provider;
use tower_sessions::Session;

use crate::{RequestSession, appstate::AppState, error::AppError};

static CSRF_KEY: OnceLock<Vec<u8>> = OnceLock::new();

/// Initialize the CSRF key from the config's `csrf_secret`.
/// `None` generates a random key and logs a warning (preserving dev behavior).
pub(crate) fn init_csrf_key(secret: &Option<String>) {
    let key = match secret {
        Some(hex_secret) => hex::decode(hex_secret).expect("CSRF_SECRET must be valid hex"),
        None => {
            warn!("generating random CSRF secret. This does not work for distributed systems");
            (0..16).map(|_| rand::random::<u8>()).collect()
        }
    };
    CSRF_KEY.set(key).expect("CSRF_KEY already initialized");
}

fn csrf_key() -> &'static [u8] {
    CSRF_KEY
        .get()
        .expect("CSRF_KEY not initialized — call init_csrf_key first")
}

fn hmac_signature(key: &[u8], msg: &str) -> String {
    use base64::prelude::{BASE64_STANDARD, Engine};
    use hmac::{Hmac, KeyInit, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(msg.as_bytes());
    let signature = mac.finalize().into_bytes();

    BASE64_STANDARD.encode(signature)
}

/// A fresh random nonce (the state's per-request secret, base64url-encoded —
/// neither it nor a PKCE verifier can contain a `+`).
fn fresh_nonce() -> String {
    use base64::prelude::{BASE64_URL_SAFE_NO_PAD, Engine};

    let random: Vec<u8> = (0..16).map(|_| rand::random()).collect();
    BASE64_URL_SAFE_NO_PAD.encode(random)
}

/// Sign a state message: the HMAC signature over `msg`, appended after a `:`.
fn signed(msg: &str) -> CsrfToken {
    let sig = hmac_signature(csrf_key(), msg);
    CsrfToken::new(msg.to_owned() + ":" + &sig)
}

/// Verify a state's signature and return its signed message (the part before
/// the `:`). A tampered or malformed state is a `BadRequest`.
fn verified_message(state: &str) -> TbResult<String> {
    let msg = state.split(':').collect::<Vec<_>>();
    if msg.len() != 2 || hmac_signature(csrf_key(), msg[0]) != msg[1] {
        return Err(Error::BadRequest(format!(
            "Bad signature for exchange request: {state}"
        )));
    }
    Ok(msg[0].to_owned())
}

/// Generate a CSRF state token encoding `path` (the OAuth return path).
pub(crate) fn gentoken(path: String) -> CsrfToken {
    let nonce = fresh_nonce();
    signed(&(path + "+" + &nonce))
}

/// Generate a CSRF state token encoding `path` (the OAuth return path) and the
/// PKCE `verifier`: the verifier round-trips through the same signed state as
/// the path, so the callback recovers it for the token exchange (stateless —
/// no server-side storage).
pub(crate) fn gentoken_pkce(path: String, verifier: &str) -> CsrfToken {
    let nonce = fresh_nonce();
    signed(&(path + "+" + &nonce + "+" + verifier))
}

/// Extract and verify the return path from a CSRF state token.
pub(crate) fn getpath(state: String) -> TbResult<String> {
    let msg = verified_message(&state)?;
    Ok(msg.split('+').next().unwrap_or("").to_owned())
}

/// Extract and verify the return path and the PKCE `verifier` from a
/// `gentoken_pkce` state token. A tampered state is rejected by the signature
/// check; a validly signed state without a verifier segment is malformed. The
/// last segment is the verifier, the second-last the nonce, and everything
/// before is the return path (rejoined with `+` — a path may contain a
/// literal `+`, which the nonce and the verifier, being base64url, never do).
pub(crate) fn getpath_and_verifier(state: String) -> TbResult<(String, String)> {
    let msg = verified_message(&state)?;
    let parts: Vec<&str> = msg.split('+').collect();
    if parts.len() < 3 {
        return Err(Error::BadRequest(format!(
            "Bad state for exchange request: {state}"
        )));
    }
    let verifier = parts[parts.len() - 1];
    let path = parts[..parts.len() - 2].join("+");
    Ok((path, verifier.to_owned()))
}

/// Complete the OAuth callback for a provider (ADR-0014): the provider-agnostic
/// `login` direct op — the provider layer's dispatch does the provider's login
/// work (the scope check, the token exchange, the athlete wire-shape parsing,
/// and the upsert), returning the identity and the admin flag. The web shell
/// forwards the opaque callback values (plus the PKCE `verifier` the signed
/// state carries, when the flow uses one) — no driver wire type crosses the
/// seam. The tower-session write stays in axum (the token is stored in the
/// provider layer's user row, not the cookie), then the redirect to the
/// return path.
pub(crate) async fn login_and_redirect(
    state: AppState,
    session: Session,
    provider: Provider,
    code: String,
    scope: String,
    path: String,
    verifier: Option<&str>,
) -> Result<Redirect, AppError> {
    let (user_id, is_admin) = state
        .provider
        .login(provider, &code, &scope, verifier)
        .await?;
    RequestSession::create_from_token(user_id, is_admin, session).await?;
    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}

/// `GET /logout`: the provider-neutral sign-out. It only tears down the
/// tower session (the provider-agnostic part of the auth lifecycle), so it
/// lives with the shared OAuth handling, not under any single provider.
pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
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

    #[test]
    fn getpath_and_verifier_recovers_path_and_verifier_from_gentoken_pkce() {
        let token = gentoken_pkce("/foo/bar".to_string(), "verifier-abc");
        assert_eq!(
            getpath_and_verifier(token.secret().clone()).expect("valid state"),
            ("/foo/bar".to_owned(), "verifier-abc".to_owned())
        );
    }

    /// A return path with a literal `+` round-trips: the nonce and the
    /// verifier are base64url (no `+`), so the parser's last-two-segments rule
    /// recovers the path with its `+` intact.
    #[test]
    fn getpath_and_verifier_recovers_a_path_with_a_literal_plus() {
        let token = gentoken_pkce("/foo+bar".to_string(), "verifier-abc");
        assert_eq!(
            getpath_and_verifier(token.secret().clone()).expect("valid state"),
            ("/foo+bar".to_owned(), "verifier-abc".to_owned())
        );
    }

    #[test]
    fn getpath_and_verifier_rejects_corrupted_signature() {
        let token = gentoken_pkce("/foo".to_string(), "verifier-abc");
        let mut state = token.secret().clone();
        state.push('x');
        assert!(matches!(
            getpath_and_verifier(state),
            Err(Error::BadRequest(_))
        ));
    }

    #[test]
    fn getpath_and_verifier_rejects_a_state_without_a_verifier() {
        // A validly signed state in the plain (verifier-less) format: the
        // signature checks out, but the verifier segment is absent.
        let nonce = fresh_nonce();
        let msg = "/foo".to_owned() + "+" + &nonce;
        let state = signed(&msg);
        assert!(matches!(
            getpath_and_verifier(state.secret().clone()),
            Err(Error::BadRequest(_))
        ));
    }
}
