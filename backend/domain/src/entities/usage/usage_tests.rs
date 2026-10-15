use crate::{TbResult, Usage, UsageId, test_support::MemStore};

/// create_usage_returns creates and reads usage correctly
#[tokio::test]
async fn create_usage_returns() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let store = &mut store;
    let usage = UsageId::new().read(store).await?;
    assert_eq!(usage.climb, 0);
    let usage2 = Usage {
        id: UsageId::new(),
        count: 1,
        climb: 2,
        descend: 3,
        ..Default::default()
    };
    let usage3 = &usage + &usage2 + &usage2;
    assert_eq!(usage3.climb, 4);
    assert_eq!(usage3.count, 2);
    assert_eq!(usage3.descend, 6);
    assert_eq!(usage3.time, 0);
    let usage3 = usage3.update(store).await?;
    let usage4 = usage3.id.read(store).await?;
    assert_eq!(usage3, usage4);
    assert_eq!(usage4 - usage3, usage);
    Usage::delete_all(store).await?;
    assert_eq!(Usage::new(usage2.id), usage2.id.read(store).await?);
    Ok(())
}
