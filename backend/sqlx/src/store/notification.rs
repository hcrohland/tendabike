use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{SqlxConn, into_domain, vec_into};
use tb_domain::{
    Notification, NotificationId, NotificationKind, NotificationLifecycle, NotificationState,
    NotificationStore, NotificationSubject, PartId, SubscriptionId, TbResult, UserId,
};

/// The database's representation of a notification.
#[derive(Clone, Debug, PartialEq, FromRow)]
struct DbNotification {
    id: Uuid,
    recipient: i32,
    lifecycle: String,
    kind: String,
    subscription_id: Option<i32>,
    part_id: Option<i32>,
    message: Option<String>,
    state: String,
    created_at: OffsetDateTime,
}

/// Maps a DB row to a domain `Notification`, resolving the subject from the
/// two nullable id columns.
impl From<DbNotification> for Notification {
    fn from(db: DbNotification) -> Self {
        let subject = match (db.subscription_id, db.part_id) {
            (Some(id), None) => NotificationSubject::Subscription(SubscriptionId::from(id)),
            (None, Some(id)) => NotificationSubject::Part(PartId::from(id)),
            _ => panic!("notification subject must have exactly one of subscription_id or part_id"),
        };
        Self {
            id: db.id.into(),
            recipient: db.recipient.into(),
            lifecycle: db.lifecycle.parse().expect("valid lifecycle"),
            kind: db.kind.parse().expect("valid kind"),
            subject,
            message: db.message,
            state: db.state.parse().expect("valid state"),
            created_at: db.created_at,
        }
    }
}

/// Serializes a lifecycle to its DB string.
fn lifecycle_to_str(l: NotificationLifecycle) -> &'static str {
    match l {
        NotificationLifecycle::Transient => "transient",
        NotificationLifecycle::Persistent => "persistent",
    }
}

/// Serializes a kind to its DB string.
fn kind_to_str(k: NotificationKind) -> &'static str {
    match k {
        NotificationKind::SubRequest => "sub_request",
        NotificationKind::SubResponse => "sub_response",
        NotificationKind::SubCancelled => "sub_cancelled",
        NotificationKind::PartRegistered => "part_registered",
        NotificationKind::PartUnregistered => "part_unregistered",
    }
}

/// Splits a `NotificationSubject` into its two nullable DB columns.
fn subject_to_cols(subject: &NotificationSubject) -> (Option<i32>, Option<i32>) {
    match subject {
        NotificationSubject::Subscription(id) => (Some(i32::from(*id)), None),
        NotificationSubject::Part(id) => (None, Some(i32::from(*id))),
    }
}

#[async_trait::async_trait]
impl<'c> NotificationStore for SqlxConn<'c> {
    async fn notification_create(
        &mut self,
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
    ) -> TbResult<Notification> {
        let id = Uuid::now_v7();
        let (sub_id, part_id) = subject_to_cols(&subject);
        sqlx::query_as!(
            DbNotification,
            "INSERT INTO notifications (id, recipient, lifecycle, kind, subscription_id, part_id, message, state, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'new', now())
             RETURNING *",
            id,
            i32::from(recipient),
            lifecycle_to_str(lifecycle),
            kind_to_str(kind),
            sub_id,
            part_id,
            message
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn notification_read(&mut self, id: NotificationId) -> TbResult<Notification> {
        // Read the notification first to determine its lifecycle.
        let id_uuid: Uuid = id.into();
        let notif: DbNotification = sqlx::query_as!(
            DbNotification,
            "SELECT * FROM notifications WHERE id = $1",
            id_uuid
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        let domain: Notification = notif.clone().into();

        // Apply the lifecycle: persistent → mark seen, transient → delete.
        match domain.lifecycle {
            NotificationLifecycle::Persistent => {
                sqlx::query!(
                    "UPDATE notifications SET state = 'seen' WHERE id = $1",
                    id_uuid
                )
                .execute(&mut **self.inner())
                .await
                .map_err(into_domain)?;
                // Return the updated notification (state = seen).
                let mut updated = domain;
                updated.state = NotificationState::Seen;
                Ok(updated)
            }
            NotificationLifecycle::Transient => {
                sqlx::query!("DELETE FROM notifications WHERE id = $1", id_uuid)
                    .execute(&mut **self.inner())
                    .await
                    .map_err(into_domain)?;
                Ok(domain)
            }
        }
    }

    async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>> {
        // Mark all persistent notifications as seen and fetch them.
        let persistent: Vec<DbNotification> = sqlx::query_as!(
            DbNotification,
            "UPDATE notifications SET state = 'seen'
             WHERE recipient = $1 AND lifecycle = 'persistent' AND state = 'new'
             RETURNING *",
            i32::from(recipient)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        // Delete all transient notifications and fetch them.
        let transient: Vec<DbNotification> = sqlx::query_as!(
            DbNotification,
            "DELETE FROM notifications
             WHERE recipient = $1 AND lifecycle = 'transient'
             RETURNING *",
            i32::from(recipient)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        let mut result: Vec<Notification> = persistent.into_iter().map(Into::into).collect();
        result.extend(transient.into_iter().map(Into::into));
        Ok(result)
    }

    async fn notification_all_for_user(
        &mut self,
        recipient: UserId,
    ) -> TbResult<Vec<Notification>> {
        let rows: Vec<DbNotification> = sqlx::query_as!(
            DbNotification,
            "SELECT * FROM notifications WHERE recipient = $1 ORDER BY created_at",
            i32::from(recipient)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(vec_into(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn date() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
    }

    fn uuid() -> Uuid {
        Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000001").unwrap()
    }

    fn db_notification(subject: NotificationSubject) -> DbNotification {
        let (sub_id, part_id) = subject_to_cols(&subject);
        let kind_str = match subject {
            NotificationSubject::Subscription(_) => "sub_request",
            NotificationSubject::Part(_) => "part_registered",
        };
        DbNotification {
            id: uuid(),
            recipient: 1,
            lifecycle: "persistent".to_string(),
            kind: kind_str.to_string(),
            subscription_id: sub_id,
            part_id,
            message: Some("test".to_string()),
            state: "new".to_string(),
            created_at: date(),
        }
    }

    #[test]
    fn notification_db_roundtrip_subscription_subject() {
        let subject = NotificationSubject::Subscription(SubscriptionId::from(42));
        let db = db_notification(subject);
        let domain = Notification::from(db);
        assert_eq!(domain.subject, subject);
        assert_eq!(domain.recipient, UserId::from(1));
        assert_eq!(domain.lifecycle, NotificationLifecycle::Persistent);
        assert_eq!(domain.kind, NotificationKind::SubRequest);
        assert_eq!(domain.state, NotificationState::New);
        assert_eq!(domain.message, Some("test".to_string()));
    }

    #[test]
    fn notification_db_roundtrip_part_subject() {
        let subject = NotificationSubject::Part(PartId::from(7));
        let mut db = db_notification(subject);
        db.kind = "part_registered".to_string();
        let domain = Notification::from(db);
        assert_eq!(domain.subject, subject);
        assert_eq!(domain.kind, NotificationKind::PartRegistered);
    }

    #[test]
    fn notification_db_roundtrip_transient_lifecycle() {
        let mut db = db_notification(NotificationSubject::Part(PartId::from(7)));
        db.lifecycle = "transient".to_string();
        db.kind = "part_registered".to_string();
        let domain = Notification::from(db);
        assert_eq!(domain.lifecycle, NotificationLifecycle::Transient);
    }

    #[test]
    fn notification_db_roundtrip_seen_state() {
        let mut db = db_notification(NotificationSubject::Subscription(SubscriptionId::from(42)));
        db.state = "seen".to_string();
        let domain = Notification::from(db);
        assert_eq!(domain.state, NotificationState::Seen);
    }

    #[test]
    fn lifecycle_from_str_roundtrip() {
        assert_eq!(
            "persistent".parse::<NotificationLifecycle>().unwrap(),
            NotificationLifecycle::Persistent
        );
        assert_eq!(
            "transient".parse::<NotificationLifecycle>().unwrap(),
            NotificationLifecycle::Transient
        );
    }

    #[test]
    fn kind_from_str_roundtrip() {
        assert_eq!(
            "sub_request".parse::<NotificationKind>().unwrap(),
            NotificationKind::SubRequest
        );
        assert_eq!(
            "part_unregistered".parse::<NotificationKind>().unwrap(),
            NotificationKind::PartUnregistered
        );
    }

    #[test]
    fn state_from_str_roundtrip() {
        assert_eq!(
            "new".parse::<NotificationState>().unwrap(),
            NotificationState::New
        );
        assert_eq!(
            "seen".parse::<NotificationState>().unwrap(),
            NotificationState::Seen
        );
    }

    #[test]
    fn subject_to_cols_subscription() {
        let (sub, part) =
            subject_to_cols(&NotificationSubject::Subscription(SubscriptionId::from(9)));
        assert_eq!(sub, Some(9));
        assert_eq!(part, None);
    }

    #[test]
    fn subject_to_cols_part() {
        let (sub, part) = subject_to_cols(&NotificationSubject::Part(PartId::from(5)));
        assert_eq!(sub, None);
        assert_eq!(part, Some(5));
    }
}
