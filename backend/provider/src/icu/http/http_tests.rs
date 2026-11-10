use super::super::test_support::{begin_test, queue, requests};
use super::*;
use tb_domain::{TbResult, UserId};

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
