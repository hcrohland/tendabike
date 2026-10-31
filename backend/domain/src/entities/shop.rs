/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

//! This module contains the tb_domain logic for shops in the Tendabike system.
//!
//! A `Shop` represents a collection of bikes owned by a shop owner. Users can register
//! their bikes to a shop, delegating maintenance to the shop owner.

use std::collections::HashMap;

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

pub mod subscription;
pub use subscription::*;

/// The database's representation of a shop.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shop {
    /// The primary key
    pub id: ShopId,
    /// The owner of the shop
    pub owner: UserId,
    /// The name of the shop
    pub name: String,
    /// Optional description of the shop
    pub description: Option<String>,
    /// If registration requests need approval
    pub auto_approve: bool,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

impl IdKeyed for Shop {
    type Key = ShopId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopId(i32);

impl ShopId {
    /// Get a shop by ID, checking that the user has access to it (ownership only)
    pub async fn get(id: i32, user: UserId, store: &mut impl ShopStore) -> TbResult<ShopId> {
        Ok(ShopId(id).check_owner(user, store).await?.id)
    }

    /// Get a shop by ID for read access (owner, or active subscriber)
    pub async fn get_for_read(
        id: i32,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<ShopId> {
        ShopId(id).check_read_access(user, store).await
    }

    pub(crate) async fn check_owner(
        &self,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        let shop = store.shop_get(*self).await?;
        if user != shop.owner {
            return Err(Error::Forbidden(
                "You must be the shop owner to access this shop".into(),
            ));
        }
        Ok(shop)
    }

    /// Check if the user has read access to this shop (owner, or active subscriber)
    pub async fn check_read_access(
        self,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<ShopId> {
        let shop = store.shop_get(self).await?;
        let is_owner = shop.owner == user;

        if is_owner || self.has_subscription(user, store).await? {
            return Ok(self);
        }

        Err(Error::Forbidden("Access denied to shop".into()))
    }

    /// Create a new shop
    pub async fn create(
        name: String,
        description: Option<String>,
        auto_approve: bool,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        store
            .shop_create(name, description, auto_approve, user)
            .await
    }

    /// Update an existing shop
    pub async fn update(
        self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Shop> {
        self.check_owner(user, store).await?;
        store
            .shop_update(self, name, description, auto_approve)
            .await
    }

    /// Delete a shop (only if it has no bikes)
    pub async fn delete(
        self,
        user: UserId,
        store: &mut (impl PartStore + ShopStore),
    ) -> TbResult<ShopId> {
        self.check_owner(user, store).await?;

        // Check if shop has any bikes
        let parts = store.shop_get_parts(self).await?;
        if !parts.is_empty() {
            return Err(Error::Conflict("Shop still has bikes assigned".into()));
        }

        store.shop_delete(self).await?;
        Ok(self)
    }

    /// Register a part (bike) to this shop
    /// Can be done by shop owner or any user with an active subscription
    /// Automatically registers all currently attached parts (cascading registration)
    /// Returns a Summary with the registered part and its attachments
    ///
    /// Side effect: a `part_registered` (transient) notification to the shop
    /// owner, naming the top-level part (a cascade of attached subparts does
    /// not fan out).
    pub async fn register_part(
        self,
        part_id: PartId,
        session: &dyn Session,
        store: &mut (impl AttachmentStore + PartStore + ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        ShopSubscription::check(self, session.user_id(), store).await?;
        let parts = parts_for_register(part_id, session, store).await?;

        // Register the parts to the shop
        let parts = store.parts_register_shop(self, parts).await?;

        let mut summary = Summary::default();
        summary.parts.upsert_all(parts);

        // Notify the shop owner (not the actor) — one notification for the
        // top-level part
        let shop = store.shop_get(self).await?;
        let notification = store
            .notification_create(
                shop.owner,
                NotificationLifecycle::Transient,
                NotificationKind::PartRegistered,
                NotificationSubject::Part(part_id),
                None,
            )
            .await?;
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    async fn has_subscription(
        self,
        user: UserId,
        store: &mut impl ShopStore,
    ) -> Result<bool, Error> {
        Ok(store.subscription_find_active(self, user).await?.is_some())
    }

    /// Unregister a part (bike) from this shop
    /// Can be done by shop owner OR part owner
    /// Returns a Summary with the unregistered part and its attachments
    ///
    /// Side effect: a `part_unregistered` (transient) notification to the
    /// part owner.
    pub async fn unregister_part(
        self,
        part_id: PartId,
        session: &dyn Session,
        store: &mut (impl AttachmentStore + PartStore + ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        let parts = parts_for_register(part_id, session, store).await?;

        let parts = store.parts_unregister_shop(parts).await?;
        // The top-level part is last in the list (parts_for_register pushes
        // subparts first, then the top-level part). Its owner is the correct
        // notification recipient.
        let part_owner = parts
            .last()
            .map(|p| p.owner)
            .ok_or_else(|| Error::AnyFailure(anyhow::anyhow!("no parts to unregister")))?;

        let mut summary = Summary::default();
        summary.parts.upsert_all(parts);

        // Notify the part owner (transient)
        let notification = store
            .notification_create(
                part_owner,
                NotificationLifecycle::Transient,
                NotificationKind::PartUnregistered,
                NotificationSubject::Part(part_id),
                None,
            )
            .await?;
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    /// Subscribe a user to this shop.
    ///
    /// Side effects (never notifies the actor of their own action):
    /// - **pending**: a `sub_request` (persistent) notification to the shop owner.
    /// - **auto-approve**: a `sub_response` (persistent) notification to the
    ///   requester, carrying the welcome message.
    pub async fn subscribe(
        self,
        message: Option<String>,
        user: UserId,
        store: &mut (impl ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        let shop = store.shop_get(self).await?;
        let sub_id = SubscriptionId::create(self, message, user, store).await?;

        let notification = if shop.auto_approve {
            let welcome = format!("Welcome to {}", shop.name);
            store
                .notification_create(
                    user,
                    NotificationLifecycle::Persistent,
                    NotificationKind::SubResponse,
                    NotificationSubject::Subscription(sub_id),
                    Some(welcome),
                )
                .await?
        } else {
            store
                .notification_create(
                    shop.owner,
                    NotificationLifecycle::Persistent,
                    NotificationKind::SubRequest,
                    NotificationSubject::Subscription(sub_id),
                    None,
                )
                .await?
        };

        let mut summary = Summary::default();
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    /// Approve a pending subscription for this shop (shop owner only).
    ///
    /// Side effect: a `sub_response` (persistent) notification to the
    /// requester, carrying the `response_message`.
    pub async fn approve_subscription(
        self,
        sub_id: SubscriptionId,
        message: Option<String>,
        user: UserId,
        store: &mut (impl ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        sub_id.approve(message.clone(), user, store).await?;

        let subscription = store.subscription_get(sub_id).await?;
        let notification = store
            .notification_create(
                subscription.user_id,
                NotificationLifecycle::Persistent,
                NotificationKind::SubResponse,
                NotificationSubject::Subscription(sub_id),
                message,
            )
            .await?;

        let mut summary = Summary::default();
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    /// Reject a pending subscription for this shop (shop owner only).
    ///
    /// Side effect: a `sub_response` (persistent) notification to the
    /// requester, carrying the `response_message`.
    pub async fn reject_subscription(
        self,
        sub_id: SubscriptionId,
        message: Option<String>,
        user: UserId,
        store: &mut (impl ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        sub_id.reject(message.clone(), user, store).await?;

        let subscription = store.subscription_get(sub_id).await?;
        let notification = store
            .notification_create(
                subscription.user_id,
                NotificationLifecycle::Persistent,
                NotificationKind::SubResponse,
                NotificationSubject::Subscription(sub_id),
                message,
            )
            .await?;

        let mut summary = Summary::default();
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    /// Cancel a subscription for this shop (subscriber only).
    ///
    /// Side effect: a `sub_cancelled` (persistent) notification to the shop
    /// owner.
    pub async fn cancel_subscription(
        self,
        sub_id: SubscriptionId,
        user: UserId,
        store: &mut (impl PartStore + ShopStore + NotificationStore),
    ) -> TbResult<Summary> {
        sub_id.cancel(user, store).await?;

        let shop = store.shop_get(self).await?;
        let notification = store
            .notification_create(
                shop.owner,
                NotificationLifecycle::Persistent,
                NotificationKind::SubCancelled,
                NotificationSubject::Subscription(sub_id),
                None,
            )
            .await?;

        let mut summary = Summary::default();
        summary.notifications.upsert(notification);
        Ok(summary)
    }

    /// Get all parts and their subparts registered to this shop
    /// Can be accessed by shop owner
    pub async fn get_parts(
        self,
        user: UserId,
        store: &mut (impl PartStore + ShopStore),
    ) -> TbResult<Vec<Part>> {
        // Check if user is shop owner OR has active subscription
        self.check_owner(user, store).await?;

        store.shop_get_parts(self).await
    }

    /// Read a shop from the database
    /// Everybiódy should be able to read this
    pub async fn read(self, store: &mut impl ShopStore) -> TbResult<Shop> {
        store.shop_get(self).await
    }
}

async fn parts_for_register(
    part_id: PartId,
    session: &dyn Session,
    store: &mut (impl AttachmentStore + PartStore + ShopStore),
) -> TbResult<Vec<PartId>> {
    part_id.checkuser(session, store).await?;
    let time = OffsetDateTime::now_utc();
    if is_attached(part_id, time, store).await? {
        return Err(Error::BadRequest(
            "You can only register parts which are not attached".to_string(),
        ));
    };
    let mut parts = subparts(part_id, time, store).await?;
    parts.push(part_id);
    Ok(parts)
}

impl Shop {
    /// Get all shops for a user
    pub async fn get_all_for_user(
        user: &UserId,
        store: &mut impl ShopStore,
    ) -> TbResult<Vec<Shop>> {
        store.shops_get_all_for_user(*user).await
    }

    /// Search for shops by name (for users to find shops to request registration)
    pub async fn search(query: &str, store: &mut impl ShopStore) -> TbResult<Vec<Shop>> {
        store.shops_search(query).await
    }

    /// Convert a list of shops to shops with owner information.
    ///
    /// `caller` is the pre-fetched [`UserPublic`] of the calling user
    /// (the `user` argument): when `Some`, it is reused for the caller's
    /// own shops instead of re-reading the row.
    pub async fn get_users(
        shops: &Vec<Shop>,
        user: &UserId,
        caller: Option<UserPublic>,
        store: &mut (impl ShopStore + UserStore),
    ) -> TbResult<Vec<UserPublic>> {
        let mut result: HashMap<UserId, UserPublic> = HashMap::new();
        for shop in shops {
            let owner = if *user == shop.owner {
                if let Some(ref c) = caller {
                    c.clone()
                } else {
                    shop.owner.get_public(store).await?
                }
            } else {
                shop.owner.get_public(store).await?
            };
            result.insert(owner.id, owner);
            if *user == shop.owner {
                shop.add_subscribers(&mut result, store).await?;
            }
        }
        Ok(result.into_values().collect())
    }

    async fn add_subscribers(
        &self,
        result: &mut HashMap<UserId, UserPublic>,
        store: &mut (impl ShopStore + UserStore),
    ) -> TbResult<()> {
        for subscription in ShopSubscription::get_for_shop(self.id, store).await? {
            let user = subscription.user_id.get_public(store).await?;
            result.insert(user.id, user);
        }
        Ok(())
    }
}

#[cfg(test)]
mod shop_tests;
