use sqlx::FromRow;
use time::OffsetDateTime;

use crate::{SqlxConn, into_domain};
use tb_domain::{Garage, GarageId, GarageSubscription, SubscriptionStatus, TbResult, UserId};

#[derive(Clone, Debug, FromRow)]
pub struct DbGarage {
    id: i32,
    owner: i32,
    name: String,
    description: Option<String>,
    created_at: OffsetDateTime,
}

impl From<Garage> for DbGarage {
    fn from(value: Garage) -> Self {
        let Garage {
            id,
            owner,
            name,
            description,
            created_at,
        } = value;
        Self {
            id: id.into(),
            owner: owner.into(),
            name,
            description,
            created_at,
        }
    }
}

impl From<DbGarage> for Garage {
    fn from(value: DbGarage) -> Self {
        let DbGarage {
            id,
            owner,
            name,
            description,
            created_at,
        } = value;
        Self {
            id: id.into(),
            owner: owner.into(),
            name,
            description,
            created_at,
        }
    }
}

#[derive(Clone, Debug, FromRow)]
pub struct DbSubscription {
    id: i32,
    garage_id: i32,
    user_id: i32,
    status: String,
    message: Option<String>,
    response_message: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<DbSubscription> for GarageSubscription {
    fn from(value: DbSubscription) -> Self {
        let DbSubscription {
            id,
            garage_id,
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
            garage_id: garage_id.into(),
            user_id: user_id.into(),
            status,
            message,
            response_message,
            created_at,
            updated_at,
        }
    }
}

#[async_session::async_trait]
impl<'c> tb_domain::GarageStore for SqlxConn<'c> {
    async fn garage_create(
        &mut self,
        name: String,
        description: Option<String>,
        owner: UserId,
    ) -> TbResult<Garage> {
        sqlx::query_as!(
            DbGarage,
            "INSERT INTO garages (owner, name, description)
             VALUES ($1, $2, $3)
             RETURNING *",
            i32::from(owner),
            name,
            description
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn garage_get(&mut self, id: GarageId) -> TbResult<Garage> {
        sqlx::query_as!(
            DbGarage,
            "SELECT * FROM garages WHERE id = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn garage_update(
        &mut self,
        id: GarageId,
        name: String,
        description: Option<String>,
    ) -> TbResult<Garage> {
        sqlx::query_as!(
            DbGarage,
            "UPDATE garages
             SET name = $2, description = $3
             WHERE id = $1
             RETURNING *",
            i32::from(id),
            name,
            description
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn garage_delete(&mut self, id: GarageId) -> TbResult<usize> {
        sqlx::query!("DELETE FROM garages WHERE id = $1", i32::from(id))
            .execute(&mut **self.inner())
            .await
            .map(|r| r.rows_affected() as usize)
            .map_err(into_domain)
    }

    async fn garages_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Garage>> {
        sqlx::query_as!(
            DbGarage,
            "SELECT DISTINCT g.* FROM garages g
             LEFT JOIN garage_subscriptions gs ON g.id = gs.garage_id AND gs.user_id = $1
             WHERE g.owner = $1
                OR (gs.status = 'active')
             ORDER BY g.name",
            i32::from(user_id)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|garages| garages.into_iter().map(Into::into).collect())
    }

    async fn garage_register_part(
        &mut self,
        garage_id: tb_domain::GarageId,
        part_id: tb_domain::PartId,
    ) -> TbResult<()> {
        sqlx::query!(
            "INSERT INTO garage_parts (garage_id, part_id)
             VALUES ($1, $2)
             ON CONFLICT (garage_id, part_id) DO NOTHING",
            i32::from(garage_id),
            i32::from(part_id)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|_| ())
    }

    async fn garage_unregister_part(
        &mut self,
        garage_id: tb_domain::GarageId,
        part_id: tb_domain::PartId,
    ) -> TbResult<()> {
        sqlx::query!(
            "DELETE FROM garage_parts WHERE garage_id = $1 AND part_id = $2",
            i32::from(garage_id),
            i32::from(part_id)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|_| ())
    }

    async fn garage_get_parts(
        &mut self,
        garage_id: tb_domain::GarageId,
    ) -> TbResult<Vec<tb_domain::PartId>> {
        sqlx::query!(
            "SELECT part_id FROM garage_parts WHERE garage_id = $1 ORDER BY registered_at",
            i32::from(garage_id)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|rows| rows.into_iter().map(|r| r.part_id.into()).collect())
    }

    async fn part_get_garage(
        &mut self,
        part_id: tb_domain::PartId,
    ) -> TbResult<Option<tb_domain::GarageId>> {
        sqlx::query!(
            "SELECT garage_id FROM garage_parts WHERE part_id = $1",
            i32::from(part_id)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|row| row.map(|r| r.garage_id.into()))
    }

    async fn garages_search(&mut self, query: &str) -> TbResult<Vec<tb_domain::Garage>> {
        let search_pattern = format!("%{}%", query);
        sqlx::query_as!(
            DbGarage,
            r#"
            SELECT DISTINCT g.id, g.owner, g.name, g.description, g.created_at
            FROM garages g
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
        .map(|garages| garages.into_iter().map(Into::into).collect())
    }

    async fn subscription_create(
        &mut self,
        garage_id: tb_domain::GarageId,
        user_id: tb_domain::UserId,
        message: Option<String>,
    ) -> TbResult<tb_domain::GarageSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "INSERT INTO garage_subscriptions (garage_id, user_id, message, status)
             VALUES ($1, $2, $3, 'pending')
             RETURNING id, garage_id, user_id, status, message, response_message, created_at, updated_at",
            i32::from(garage_id),
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
    ) -> TbResult<tb_domain::GarageSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
             FROM garage_subscriptions WHERE id = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn subscription_find_active(
        &mut self,
        garage_id: tb_domain::GarageId,
        user_id: tb_domain::UserId,
    ) -> TbResult<Option<tb_domain::GarageSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
             FROM garage_subscriptions
             WHERE garage_id = $1 AND user_id = $2 AND status = 'active'",
            i32::from(garage_id),
            i32::from(user_id)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|opt| opt.map(Into::into))
    }

    async fn subscription_find_pending(
        &mut self,
        garage_id: tb_domain::GarageId,
        user_id: tb_domain::UserId,
    ) -> TbResult<Option<tb_domain::GarageSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
             FROM garage_subscriptions
             WHERE garage_id = $1 AND user_id = $2 AND status = 'pending'",
            i32::from(garage_id),
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
    ) -> TbResult<tb_domain::GarageSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "UPDATE garage_subscriptions SET status = $2
             WHERE id = $1
             RETURNING id, garage_id, user_id, status, message, response_message, created_at, updated_at",
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
    ) -> TbResult<tb_domain::GarageSubscription> {
        sqlx::query_as!(
            DbSubscription,
            "UPDATE garage_subscriptions SET status = $2, response_message = $3
             WHERE id = $1
             RETURNING id, garage_id, user_id, status, message, response_message, created_at, updated_at",
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
            "DELETE FROM garage_subscriptions WHERE id = $1",
            i32::from(id)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(|_| ())
    }

    async fn subscriptions_for_garage(
        &mut self,
        garage_id: tb_domain::GarageId,
        status: Option<tb_domain::SubscriptionStatus>,
    ) -> TbResult<Vec<tb_domain::GarageSubscription>> {
        match status {
            Some(status) => sqlx::query_as!(
                DbSubscription,
                "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
                 FROM garage_subscriptions
                 WHERE garage_id = $1 AND status = $2
                 ORDER BY created_at DESC",
                i32::from(garage_id),
                status.to_string()
            )
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(|subscriptions| subscriptions.into_iter().map(Into::into).collect()),
            None => sqlx::query_as!(
                DbSubscription,
                "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
                 FROM garage_subscriptions
                 WHERE garage_id = $1
                 ORDER BY created_at DESC",
                i32::from(garage_id)
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
    ) -> TbResult<Vec<tb_domain::GarageSubscription>> {
        sqlx::query_as!(
            DbSubscription,
            "SELECT id, garage_id, user_id, status, message, response_message, created_at, updated_at
             FROM garage_subscriptions
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
