//! Test-only doubles for the transaction seam (ADR-0005, executable spec #446 §4.1).
//!
//! `FakeConn` implements the nine `Store` sub-traits (every method
//! `unimplemented!()` — they are never called) and `Txn` (consuming
//! `commit`/`rollback` that return a fixed `Ok(())`); `FakeSource` is a
//! `TxnSource` whose `Conn` is the `FakeConn`. The fakes only prove the trait
//! bounds compose — they are test-only, not public API.

#![allow(clippy::too_many_arguments)]

use time::OffsetDateTime;

use crate::{Txn, TxnSource};
use tb_domain::*;

/// A fake connection: the nine `Store` sub-traits + the marker + `Txn`.
struct FakeConn;

impl Store for FakeConn {}

#[async_trait::async_trait]
impl PartStore for FakeConn {
    async fn partid_get_part(&mut self, _pid: PartId) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_get_all_for_userid(&mut self, _uid: &UserId) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn part_create(
        &mut self,
        _what: PartTypeId,
        _name: String,
        _vendor: String,
        _model: String,
        _purchase: OffsetDateTime,
        _source: Option<String>,
        _usage: UsageId,
        _owner: UserId,
        _shop: Option<ShopId>,
    ) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_update(&mut self, _part: Part) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_delete(&mut self, _part: PartId) -> TbResult<PartId> {
        unimplemented!()
    }
    async fn parts_delete(&mut self, _parts: &[Part]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn partid_get_by_source(&mut self, _strava_id: &str) -> TbResult<Option<PartId>> {
        unimplemented!()
    }
    async fn parts_register_shop(
        &mut self,
        _shop_id: ShopId,
        _part_id: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn parts_unregister_shop(&mut self, _part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn shop_get_parts(&mut self, _shop_id: ShopId) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl UserStore for FakeConn {
    async fn get(&mut self, _uid: UserId) -> TbResult<User> {
        unimplemented!()
    }
    async fn create(
        &mut self,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        unimplemented!()
    }
    async fn update(
        &mut self,
        _uid: &UserId,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        unimplemented!()
    }
    async fn user_delete(&mut self, _user: &UserId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn update_onboarding_status(
        &mut self,
        _uid: &UserId,
        _status: OnboardingStatus,
    ) -> TbResult<User> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ShopStore for FakeConn {
    async fn shop_create(
        &mut self,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
        _owner: UserId,
    ) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_get(&mut self, _id: ShopId) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_update(
        &mut self,
        _id: ShopId,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
    ) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_delete(&mut self, _id: ShopId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn shops_get_all_for_user(&mut self, _user_id: UserId) -> TbResult<Vec<Shop>> {
        unimplemented!()
    }
    async fn shops_search(&mut self, _query: &str) -> TbResult<Vec<Shop>> {
        unimplemented!()
    }
    async fn subscription_create(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
        _message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_get(&mut self, _id: SubscriptionId) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_find_active(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscription_find_pending(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscription_update_status(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_approve(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
        _response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_delete(&mut self, _id: SubscriptionId) -> TbResult<()> {
        unimplemented!()
    }
    async fn subscriptions_for_shop(
        &mut self,
        _shop_id: ShopId,
    ) -> TbResult<Vec<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscriptions_for_user(
        &mut self,
        _user_id: UserId,
    ) -> TbResult<Vec<ShopSubscription>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ActivityStore for FakeConn {
    async fn activity_create(&mut self, _act: Activity) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_read_by_id(&mut self, _aid: ActivityId) -> TbResult<Option<Activity>> {
        unimplemented!()
    }
    async fn activity_update(&mut self, _act: Activity) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_delete(&mut self, _aid: ActivityId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn activities_delete(&mut self, _activities: &[Activity]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn get_all(&mut self, _uid: &UserId) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn activities_find_by_gear_and_time(
        &mut self,
        _part: PartId,
        _begin: OffsetDateTime,
        _end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn get_by_user_and_time(
        &mut self,
        _uid: UserId,
        _rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_set_gear_if_null(
        &mut self,
        _user: UserId,
        _types: Vec<ActTypeId>,
        _partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl AttachmentStore for FakeConn {
    async fn attachment_create(&mut self, _att: Attachment) -> TbResult<Attachment> {
        unimplemented!()
    }
    async fn delete(&mut self, _att: Attachment) -> TbResult<Attachment> {
        unimplemented!()
    }
    async fn attachments_delete_by_parts(&mut self, _parts: &[Part]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn attachment_get_by_gear_and_time(
        &mut self,
        _act_gear: PartId,
        _start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachments_all_by_part(&mut self, _id: PartId) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachment_get_by_part_and_time(
        &mut self,
        _pid: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        _types: Vec<PartTypeId>,
        _gear: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        _what: PartTypeId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_successor(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
        _what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        _part_id: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_part_attached_already(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl PartNoteStore for FakeConn {
    async fn partnote_create_text(
        &mut self,
        _part: PartId,
        _name: String,
        _created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_create_file(
        &mut self,
        _part: PartId,
        _name: String,
        _mime: String,
        _filename: Option<String>,
        _size: i64,
        _data: Vec<u8>,
        _created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_all_by_part(&mut self, _part: PartId) -> TbResult<Vec<PartNote>> {
        unimplemented!()
    }
    async fn partnote_get(&mut self, _id: PartNoteId) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_file(&mut self, _id: PartNoteId) -> TbResult<Vec<u8>> {
        unimplemented!()
    }
    async fn partnote_update_text(&mut self, _id: PartNoteId, _name: String) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_update_file(
        &mut self,
        _id: PartNoteId,
        _name: String,
        _mime: String,
        _filename: Option<String>,
        _size: i64,
        _data: Vec<u8>,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_remove_file(&mut self, _id: PartNoteId) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_delete(&mut self, _id: PartNoteId) -> TbResult<PartNoteId> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl UsageStore for FakeConn {
    async fn get(&mut self, _uid: UsageId) -> TbResult<Option<Usage>> {
        unimplemented!()
    }
    async fn update<U>(&mut self, _usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        unimplemented!()
    }
    async fn delete(&mut self, _usage: UsageId) -> TbResult<Usage> {
        unimplemented!()
    }
    async fn usages_delete(&mut self, _usages: &[Usage]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn delete_all(&mut self) -> TbResult<usize> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ServiceStore for FakeConn {
    async fn create(&mut self, _service: Service) -> TbResult<Service> {
        unimplemented!()
    }
    async fn get(&mut self, _service: ServiceId) -> TbResult<Service> {
        unimplemented!()
    }
    async fn update(&mut self, _service: Service) -> TbResult<Service> {
        unimplemented!()
    }
    async fn delete(&mut self, _service: ServiceId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn services_delete(&mut self, _services: &[Service]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn services_by_part(&mut self, _part: PartId) -> TbResult<Vec<Service>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ServicePlanStore for FakeConn {
    async fn create(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn get(&mut self, _plan: ServicePlanId) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn plan_update(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn delete(&mut self, _plan: ServicePlanId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn serviceplans_delete(&mut self, _serviceplans: &[ServicePlan]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn by_part(&mut self, _part: PartId) -> TbResult<Vec<ServicePlan>> {
        unimplemented!()
    }
    async fn by_user(&mut self, _uid: UserId) -> TbResult<Vec<ServicePlan>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl Txn for FakeConn {
    async fn commit(self) -> TbResult<()> {
        Ok(())
    }
    async fn rollback(self) -> TbResult<()> {
        Ok(())
    }
}

/// A fake source: opens a transaction by returning a fresh `FakeConn`.
struct FakeSource;

#[async_trait::async_trait]
impl TxnSource for FakeSource {
    type Conn = FakeConn;

    async fn begin(&self) -> TbResult<Self::Conn> {
        Ok(FakeConn)
    }
}

/// The seam's contract: a generic caller bounded only on `TxnSource` can
/// `begin` a connection and then close it with `commit` or `rollback` — the
/// `Conn: Store + Txn` linkage is what makes this type-check.
async fn run_one<S: TxnSource>(source: &S, commit: bool) -> TbResult<()> {
    let conn = source.begin().await?;
    if commit {
        conn.commit().await
    } else {
        conn.rollback().await
    }
}

#[tokio::test]
async fn a_generic_source_can_begin_then_commit_and_rollback() -> TbResult<()> {
    let source = FakeSource;
    run_one(&source, true).await?;
    run_one(&source, false).await?;
    Ok(())
}
