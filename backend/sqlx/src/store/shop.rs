use sqlx::FromRow;
use time::OffsetDateTime;

use crate::{SqlxConn, into_domain};
use tb_domain::{Shop, ShopId, ShopSubscription, SubscriptionStatus, TbResult, UserId};

#[derive(Clone, Debug, FromRow)]
pub struct DbShop {
    id: i32,
    owner: i32,
    name: String,
    description: Option<String>,
    created_at: OffsetDateTime,
    auto_approve: bool,
}

impl From<Shop> for DbShop {
    fn from(value: Shop) -> Self {
        let Shop {
            id,
            owner,
            name,
            description,
            auto_approve,
            created_at,
        } = value;
        Self {
            id: id.into(),
            owner: owner.into(),
            name,
            description,
            auto_approve,
            created_at,
        }
    }
}

impl From<DbShop> for Shop {
    fn from(value: DbShop) -> Self {
        let DbShop {
            id,
            owner,
            name,
            description,
            auto_approve,
            created_at,
        } = value;
        Self {
            id: id.into(),
            owner: owner.into(),
            name,
            description,
            auto_approve,
            created_at,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct DbSubscription {
    id: i32,
    shop_id: i32,
    user_id: i32,
    status: String,
    message: Option<String>,
    response_message: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<DbSubscription> for ShopSubscription {
    fn from(value: DbSubscription) -> Self {
        let DbSubscription {
            id,
            shop_id,
            user_id,
            status,
            message,
            response_message,
            created_at,
            updated_at,
        } = value;

        let status = match status.as_str() {
            "active" => SubscriptionStatus::Active,
            "rejected" => SubscriptionStatus::Rejected,
            "cancelled" => SubscriptionStatus::Cancelled,
            _ => SubscriptionStatus::Pending,
        };

        Self {
            id: id.into(),
            shop_id: shop_id.into(),
            user_id: user_id.into(),
            status,
            message,
            response_message,
            created_at,
            updated_at,
        }
    }
}

#[async_trait::async_trait]
impl<'c> tb_domain::ShopStore for SqlxConn<'c> {
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        owner: UserId,
    ) -> TbResult<Shop> {
        sqlx::query_as!(
            DbShop,
            "INSERT INTO shops (owner, name, description, auto_approve)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
            i32::from(owner),
            name,
            description,
            auto_approve
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> {
        sqlx::query_as!(DbShop, "SELECT * FROM shops WHERE id = $1", i32::from(id))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    ) -> TbResult<Shop> {
        sqlx::query_as!(
            DbShop,
            "UPDATE shops
             SET name = $2, description = $3, auto_approve = $4
             WHERE id = $1
             RETURNING *",
            i32::from(id),
            name,
            description,
            auto_approve
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> {
        sqlx::query!("DELETE FROM shops WHERE id = $1", i32::from(id))
            .execute(&mut **self.inner())
            .await
            .map(|r| r.rows_affected() as usize)
            .map_err(into_domain)
    }

    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> {
        sqlx::query_as!(
            DbShop,
            "SELECT DISTINCT g.* FROM shops g
             LEFT JOIN shop_subscriptions gs ON g.id = gs.shop_id AND gs.user_id = $1
             WHERE g.owner = $1
                OR (gs.status = 'active')
             ORDER BY g.name",
            i32::from(user_id)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|shops| shops.into_iter().map(Into::into).collect())
    }

    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<tb_domain::Shop>> {
        let search_pattern = format!("%{}%", query);
        sqlx::query_as!(
            DbShop,
            r#"
            SELECT DISTINCT g.id, g.owner, g.name, g.description, g.created_at, g.auto_approve
            FROM shops g
            LEFT JOIN users u ON g.owner = u.id
            WHERE g.name ILIKE $1
               OR COALESCE(u.firstname, '') ILIKE $1
               OR COALESCE(u.name, '') ILIKE $1
               OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
               OR CONCAT(g.name, ' ', COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
               OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, ''), ' ', g.name) ILIKE $1
            ORDER BY g.name
            LIMIT 50
            "#,
            search_pattern
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|shops| shops.into_iter().map(Into::into).collect())
    }

    async fn subscription_create(
        &mut self,
        shop_id: tb_domain::ShopId,
        user_id: tb_domain::UserId,
        message: Option<String>,
    ) -> TbResult<tb_domain::ShopSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "INSERT INTO shop_subscriptions (shop_id, user_id, message, status)
             VALUES ($1, $2, $3, 'pending')
             RETURNING id, shop_id, user_id, status, message, response_message, created_at, updated_at",
            i32::from(shop_id),
            i32::from(user_id),
            message
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn subscription_get(
        &mut self,
        id: tb_domain::SubscriptionId,
    ) -> TbResult<tb_domain::ShopSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
             FROM shop_subscriptions WHERE id = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn subscription_find_active(
        &mut self,
        shop_id: tb_domain::ShopId,
        user_id: tb_domain::UserId,
    ) -> TbResult<Option<tb_domain::ShopSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
             FROM shop_subscriptions
             WHERE shop_id = $1 AND user_id = $2 AND status = 'active'",
            i32::from(shop_id),
            i32::from(user_id)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|opt| opt.map(Into::into))
    }

    async fn subscription_find_pending(
        &mut self,
        shop_id: tb_domain::ShopId,
        user_id: tb_domain::UserId,
    ) -> TbResult<Option<tb_domain::ShopSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
             FROM shop_subscriptions
             WHERE shop_id = $1 AND user_id = $2 AND status = 'pending'",
            i32::from(shop_id),
            i32::from(user_id)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|opt| opt.map(Into::into))
    }

    async fn subscription_update_status(
        &mut self,
        id: tb_domain::SubscriptionId,
        status: tb_domain::SubscriptionStatus,
    ) -> TbResult<tb_domain::ShopSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "UPDATE shop_subscriptions SET status = $2
             WHERE id = $1
             RETURNING id, shop_id, user_id, status, message, response_message, created_at, updated_at",
            i32::from(id),
            status.to_string()
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn subscription_approve(
        &mut self,
        id: tb_domain::SubscriptionId,
        status: tb_domain::SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<tb_domain::ShopSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "UPDATE shop_subscriptions SET status = $2, response_message = $3
             WHERE id = $1
             RETURNING id, shop_id, user_id, status, message, response_message, created_at, updated_at",
            i32::from(id),
            status.to_string(),
            response_message
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn subscription_delete(&mut self, id: tb_domain::SubscriptionId) -> TbResult<()> {
        sqlx::query!(
            "DELETE FROM shop_subscriptions WHERE id = $1",
            i32::from(id)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|_| ())
    }

    async fn subscriptions_for_shop(
        &mut self,
        shop_id: tb_domain::ShopId,
        status: Option<tb_domain::SubscriptionStatus>,
    ) -> TbResult<Vec<tb_domain::ShopSubscription>> {
        match status {
            Some(status) => sqlx::query_as!(
                DbSubscription,
                "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
                 FROM shop_subscriptions
                 WHERE shop_id = $1 AND status = $2
                 ORDER BY created_at DESC",
                i32::from(shop_id),
                status.to_string()
            )
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(|subscriptions| subscriptions.into_iter().map(Into::into).collect()),
            None => sqlx::query_as!(
                DbSubscription,
                "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
                 FROM shop_subscriptions
                 WHERE shop_id = $1
                 ORDER BY created_at DESC",
                i32::from(shop_id)
            )
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(|subscriptions| subscriptions.into_iter().map(Into::into).collect()),
        }
    }

    async fn subscriptions_for_user(
        &mut self,
        user_id: tb_domain::UserId,
    ) -> TbResult<Vec<tb_domain::ShopSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, shop_id, user_id, status, message, response_message, created_at, updated_at
             FROM shop_subscriptions
             WHERE user_id = $1
             ORDER BY created_at DESC",
            i32::from(user_id)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|subscriptions| subscriptions.into_iter().map(Into::into).collect())
    }
}
