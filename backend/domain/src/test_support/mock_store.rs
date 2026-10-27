//! A `mockall` `mock!` witness for the `Store` seam (issue #630).
//!
//! [`MockStore`] is a single type implementing the full `Store` seam — the nine
//! sub-traits (`PartStore`, `UserStore`, `ShopStore`, `ActivityStore`,
//! `AttachmentStore`, `PartNoteStore`, `UsageStore`, `ServiceStore`,
//! `ServicePlanStore`) plus the method-free [`Store`] marker. It is the store
//! the re-based domain-logic tests run their mock-runnable cases against: the
//! domain logic runs for real, the store is a canned `mock!` dependency.
//!
//! `#[async_trait]` on each impl block is the trick that makes the `async fn`
//! methods desugar correctly in the generated code (see
//! `docs/research/mockall-conn-witness.md` §1). The seam is collision-free
//! (post-recipe: renamed methods, `usages_update(&[Usage])`, `stravauser_update_token`
//! owning its token), so `mock!`'s flat `expect_<name>` API can express every
//! method on one type.
//!
//! The declared struct name is `Store`, so `mock!` emits `MockStore`; the impl
//! blocks reference `Store` (the macro rewrites it to `MockStore`), and the
//! method-free marker `impl Store for Store` becomes `impl Store for MockStore`.
//!
//! Test-support only: this module (and its `pub` items) is gated behind
//! `cfg(test)` / the `test-support` feature, so no production `pub` surface is
//! widened. `mockall` is a dev-dependency of the consuming crates only.

use time::OffsetDateTime;

use crate::*;

mockall::mock! {
    pub Store {}

    #[async_trait::async_trait]
    impl PartStore for Store {
        async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part>;
        async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>>;
        async fn part_create(
            &mut self,
            what: PartTypeId,
            name: String,
            vendor: String,
            model: String,
            purchase: OffsetDateTime,
            source: Option<String>,
            usage: UsageId,
            owner: UserId,
            shop: Option<ShopId>,
        ) -> TbResult<Part>;
        async fn part_update(&mut self, part: Part) -> TbResult<Part>;
        async fn part_delete(&mut self, part: PartId) -> TbResult<PartId>;
        async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>>;
        async fn parts_register_shop(
            &mut self,
            shop_id: ShopId,
            part_id: Vec<PartId>,
        ) -> TbResult<Vec<Part>>;
        async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>>;
        async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>>;
    }

    #[async_trait::async_trait]
    impl UserStore for Store {
        async fn user_get(&mut self, uid: UserId) -> TbResult<User>;
        async fn user_create(
            &mut self,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_update(
            &mut self,
            uid: &UserId,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_delete(&mut self, user: &UserId) -> TbResult<usize>;
        async fn update_onboarding_status(
            &mut self,
            uid: &UserId,
            status: OnboardingStatus,
        ) -> TbResult<User>;
    }

    #[async_trait::async_trait]
    impl ShopStore for Store {
        async fn shop_create(
            &mut self,
            name: String,
            description: Option<String>,
            auto_approve: bool,
            owner: UserId,
        ) -> TbResult<Shop>;
        async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop>;
        async fn shop_update(
            &mut self,
            id: ShopId,
            name: String,
            description: Option<String>,
            auto_approve: bool,
        ) -> TbResult<Shop>;
        async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize>;
        async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>>;
        async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>>;
        async fn subscription_create(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
            message: Option<String>,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription>;
        async fn subscription_find_active(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_find_pending(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_update_status(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_approve(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
            response_message: Option<String>,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()>;
        async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>>;
        async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>>;
    }

    #[async_trait::async_trait]
    impl ActivityStore for Store {
        async fn activity_create(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>>;
        async fn activity_update(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize>;
        async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize>;
        async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>>;
        async fn activities_find_by_gear_and_time(
            &mut self,
            part: PartId,
            begin: OffsetDateTime,
            end: OffsetDateTime,
        ) -> TbResult<Vec<Activity>>;
        async fn get_by_user_and_time(
            &mut self,
            uid: UserId,
            rstart: OffsetDateTime,
        ) -> TbResult<Activity>;
        async fn activity_set_gear_if_null(
            &mut self,
            user: UserId,
            types: Vec<ActTypeId>,
            partid: &PartId,
        ) -> TbResult<Vec<Activity>>;
        async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>>;
    }

    #[async_trait::async_trait]
    impl AttachmentStore for Store {
        async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachment_delete(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachments_delete_by_parts(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn attachment_get_by_gear_and_time(
            &mut self,
            act_gear: PartId,
            start: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>>;
        async fn attachment_get_by_part_and_time(
            &mut self,
            pid: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn assembly_get_by_types_time_and_gear(
            &mut self,
            types: Vec<PartTypeId>,
            gear: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachment_find_part_of_type_at_hook_and_time(
            &mut self,
            what: PartTypeId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_successor(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
            what: PartTypeId,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_later_attachment_for_part(
            &mut self,
            part_id: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_part_attached_already(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
    }

    #[async_trait::async_trait]
    impl PartNoteStore for Store {
        async fn partnote_create_text(
            &mut self,
            part: PartId,
            name: String,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_create_file(
            &mut self,
            part: PartId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>>;
        async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>>;
        async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote>;
        async fn partnote_update_file(
            &mut self,
            id: PartNoteId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
        ) -> TbResult<PartNote>;
        async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId>;
    }

    #[async_trait::async_trait]
    impl UsageStore for Store {
        async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>>;
        async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize>;
        async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage>;
        async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize>;
        async fn delete_all(&mut self) -> TbResult<usize>;
    }

    #[async_trait::async_trait]
    impl ServiceStore for Store {
        async fn service_create(&mut self, service: Service) -> TbResult<Service>;
        async fn service_get(&mut self, service: ServiceId) -> TbResult<Service>;
        async fn service_update(&mut self, service: Service) -> TbResult<Service>;
        async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize>;
        async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize>;
        async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>>;
    }

    #[async_trait::async_trait]
    impl ServicePlanStore for Store {
        async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
        async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize>;
        async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize>;
        async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
        async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>>;
    }

    impl Store for Store {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A consumer requiring the full `Store` seam: proving `MockStore`
    /// satisfies the bound (all nine sub-traits + the marker + `Send`) is the
    /// acceptance bar for the witness.
    async fn read_usage<S: Store>(store: &mut S, uid: UsageId) -> TbResult<Option<Usage>> {
        store.usage_get(uid).await
    }

    #[tokio::test]
    async fn mock_store_serves_canned_outcome_through_the_store_seam() -> TbResult<()> {
        let mut mock = MockStore::new();
        mock.expect_usage_get().returning(|_uid| Ok(None));

        let got = read_usage(&mut mock, UsageId::default()).await?;
        assert!(got.is_none());
        Ok(())
    }
}
