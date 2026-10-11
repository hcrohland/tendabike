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

/// Usage::add combines two usages correctly
#[test]
fn usage_add_two_usages() {
    let u1 = Usage {
        id: UsageId::default(),
        time: 1000,
        distance: 5000,
        climb: 100,
        descend: 80,
        energy: 500,
        count: 1,
    };
    let u2 = Usage {
        id: UsageId::default(),
        time: 2000,
        distance: 15000,
        climb: 300,
        descend: 200,
        energy: 1500,
        count: 2,
    };

    let combined = &u1 + &u2;
    assert_eq!(combined.time, 3000);
    assert_eq!(combined.distance, 20000);
    assert_eq!(combined.climb, 400);
    assert_eq!(combined.descend, 280);
    assert_eq!(combined.energy, 2000);
    assert_eq!(combined.count, 3);
}

/// Usage::sub correctly subtracts two usages
#[test]
fn usage_subtracts_correctly() {
    let u1 = Usage {
        id: UsageId::default(),
        time: 3000,
        distance: 20000,
        climb: 400,
        descend: 280,
        energy: 2000,
        count: 3,
    };
    let u2 = Usage {
        id: UsageId::default(),
        time: 1000,
        distance: 5000,
        climb: 100,
        descend: 80,
        energy: 500,
        count: 1,
    };

    let diff = u1 - u2;
    assert_eq!(diff.time, 2000);
    assert_eq!(diff.distance, 15000);
    assert_eq!(diff.climb, 300);
    assert_eq!(diff.descend, 200);
    assert_eq!(diff.energy, 1500);
    assert_eq!(diff.count, 2);
}

/// Usage::neg produces fully inverted usage
#[test]
fn usage_negation_produces_inverted() {
    let u = Usage {
        id: UsageId::default(),
        time: 1000,
        distance: 5000,
        climb: 100,
        descend: 80,
        energy: 500,
        count: 1,
    };

    let neg_u = -u;
    assert_eq!(neg_u.time, -1000);
    assert_eq!(neg_u.distance, -5000);
    assert_eq!(neg_u.climb, -100);
    assert_eq!(neg_u.descend, -80);
    assert_eq!(neg_u.energy, -500);
    assert_eq!(neg_u.count, -1);
}

/// Usage add_vec adds a single usage to all elements
#[test]
fn usage_add_vec_adds_single() {
    let u1 = Usage {
        id: UsageId::default(),
        time: 1000,
        distance: 5000,
        climb: 100,
        descend: 80,
        energy: 500,
        count: 1,
    };
    let u2 = Usage {
        id: UsageId::default(),
        time: 2000,
        distance: 15000,
        climb: 300,
        descend: 200,
        energy: 1500,
        count: 2,
    };
    let increment = Usage {
        id: UsageId::default(),
        time: 500,
        distance: 2000,
        climb: 100,
        descend: 50,
        energy: 250,
        count: 1,
    };

    let result = vec![u1, u2] + &increment;
    assert_eq!(result[0].time, 1500);
    assert_eq!(result[1].time, 2500);
    assert_eq!(result[0].distance, 7000);
    assert_eq!(result[1].distance, 17000);
}
