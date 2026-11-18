use super::*;
use crate::facade::{TbResult, UserId};
use crate::http::BearerGetMapping;
use crate::test_support::{begin_test, queue, requests};

/// `request_json` reads the durable token from `icu_users` and replays the
/// canned response (the offline seam): the URI is recorded and the body
/// parsed.
#[tokio::test]
async fn request_json_replays_canned_response() -> TbResult<()> {
    begin_test();
    let mut store = super::super::traits::MockIcuDataStore::new();
    store.expect_icuser_get_by_tbid().returning(|_id| {
        Ok(super::super::test_support::icu_user(
            UserId::from(1),
            "a1",
            true,
        ))
    });
    queue("/activity/10", r#"{"id":"10"}"#);
    let v: serde_json::Value = request_json(UserId::from(1), "/activity/10", &mut store).await?;
    assert_eq!(v["id"], "10");
    assert_eq!(requests(), vec!["/activity/10".to_string()]);
    Ok(())
}

/// intervals.icu's status mapping: the provider-specific wording, and the
/// `Permanent` arm reads the response body into the error detail.
#[test]
fn icu_mapping_carries_icu_wording_and_reads_the_body() {
    let m = IcuMapping;
    assert!(m.reads_body());
    assert!(matches!(
        m.map(StatusCode::TOO_MANY_REQUESTS, "/activity/10", None),
        Error::TryAgain(_)
    ));
    let not_auth = m.map(StatusCode::UNAUTHORIZED, "/activity/10", None);
    assert!(
        matches!(&not_auth, Error::NotAuth(msg) if msg.contains("intervals.icu") && msg.contains("/activity/10"))
    );
    let perm = m.map(
        StatusCode::NOT_FOUND,
        "/activity/10",
        Some("quota exceeded".into()),
    );
    assert!(
        matches!(&perm, Error::BadRequest(msg) if msg.contains("intervals.icu request error") && msg.contains("quota exceeded"))
    );
}
