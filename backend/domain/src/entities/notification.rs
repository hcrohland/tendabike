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

//! This module contains the domain logic for notifications.
//!
//! A [`Notification`] is a message addressed to a single recipient, created
//! as a side effect of a domain operation — never directly by the user
//! (issue #586). It has a kind and a lifecycle.
//!
//! **Lifecycles:**
//! - **Persistent** (subscription kinds): a read marks it `seen` and it stays
//!   in the history.
//! - **Transient** (part kinds): a read deletes the row, and the frame
//!   carries its tombstone.
//!
//! Notifications ride the [`Summary`] as its own map (the tenth,
//! `notifications`): the operation's transaction creates the row and adds it
//! to the returned `Summary`, like any other side effect.

use derive_more::{Display, From, Into};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::*;

/// A message addressed to a single recipient, created as a side effect of a
/// domain operation — never directly by the user (issue #586).
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    /// The primary key
    pub id: NotificationId,
    /// The recipient
    pub recipient: UserId,
    /// The lifecycle: decides what a read does
    pub lifecycle: NotificationLifecycle,
    /// The kind of notification
    pub kind: NotificationKind,
    /// The subject reference (subscription or part)
    pub subject: NotificationSubject,
    /// Optional user-generated text (e.g. a rejection reply)
    pub message: Option<String>,
    /// The read state
    pub state: NotificationState,
    /// When the notification was created
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

impl IdKeyed for Notification {
    type Key = NotificationId;
    fn key(&self) -> Self::Key {
        self.id
    }
}

/// The notification's primary key: a Uuid v7.
#[derive(Clone, Copy, Debug, Display, From, Into, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationId(Uuid);

impl std::str::FromStr for NotificationId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Uuid>().map(Self).map_err(|e| e.to_string())
    }
}

impl NotificationId {
    /// Mints a fresh v7 id.
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Uuid::now_v7().into()
    }

    /// Read a notification: applies its lifecycle (persistent → mark seen,
    /// transient → delete) and returns the [`Summary`] carrying the result.
    ///
    /// The recipient guard (ADR-0008): the store filters by recipient, so a
    /// non-recipient gets `None` → `NotFound` (no existence leak).
    ///
    /// A persistent notification is upserted into the summary (now `seen`);
    /// a transient notification is deleted and its id tombstoned.
    pub async fn read(
        self,
        recipient: UserId,
        store: &mut impl NotificationStore,
    ) -> TbResult<Summary> {
        let notification = store
            .notification_read(self, recipient)
            .await?
            .ok_or_else(|| Error::NotFound(self.to_string()))?;
        let mut summary = Summary::default();
        match notification.lifecycle {
            NotificationLifecycle::Persistent => {
                summary.notifications.upsert(notification);
            }
            NotificationLifecycle::Transient => {
                summary.notifications.tombstone(notification.id);
            }
        }
        Ok(summary)
    }
}

/// The notification's lifecycle: decides what a read does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLifecycle {
    /// A read deletes the row; the frame carries its tombstone.
    Transient,
    /// A read marks it `seen`; it stays in the history.
    Persistent,
}

impl std::str::FromStr for NotificationLifecycle {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "transient" => Ok(Self::Transient),
            "persistent" => Ok(Self::Persistent),
            _ => Err(format!("unknown lifecycle: {s}")),
        }
    }
}

/// The kind of notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    /// A subscription request was made.
    SubRequest,
    /// A subscription request was responded to (approved or rejected).
    SubResponse,
    /// A subscription was cancelled.
    SubCancelled,
    /// A part was registered to a shop.
    PartRegistered,
    /// A part was unregistered from a shop.
    PartUnregistered,
}

impl std::fmt::Display for NotificationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubRequest => write!(f, "sub_request"),
            Self::SubResponse => write!(f, "sub_response"),
            Self::SubCancelled => write!(f, "sub_cancelled"),
            Self::PartRegistered => write!(f, "part_registered"),
            Self::PartUnregistered => write!(f, "part_unregistered"),
        }
    }
}

impl std::str::FromStr for NotificationKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sub_request" => Ok(Self::SubRequest),
            "sub_response" => Ok(Self::SubResponse),
            "sub_cancelled" => Ok(Self::SubCancelled),
            "part_registered" => Ok(Self::PartRegistered),
            "part_unregistered" => Ok(Self::PartUnregistered),
            _ => Err(format!("unknown kind: {s}")),
        }
    }
}

/// The subject of a notification: the entity it refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum NotificationSubject {
    /// The notification refers to a subscription.
    Subscription(SubscriptionId),
    /// The notification refers to a part.
    Part(PartId),
}

/// The read state of a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationState {
    /// Not yet read.
    New,
    /// Read (persistent only; transient rows are deleted on read).
    Seen,
}

impl std::str::FromStr for NotificationState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "new" => Ok(Self::New),
            "seen" => Ok(Self::Seen),
            _ => Err(format!("unknown state: {s}")),
        }
    }
}

impl Notification {
    /// Read all of a user's notifications: applies each one's lifecycle
    /// (persistent → mark seen, transient → delete) and returns the
    /// [`Summary`] carrying the results.
    ///
    /// Persistent notifications are upserted (now `seen`); transient
    /// notifications are deleted and their ids tombstoned.
    pub async fn read_all(
        recipient: UserId,
        store: &mut impl NotificationStore,
    ) -> TbResult<Summary> {
        let notifications = store.notification_read_all(recipient).await?;
        let mut summary = Summary::default();
        for notification in notifications {
            match notification.lifecycle {
                NotificationLifecycle::Persistent => {
                    summary.notifications.upsert(notification);
                }
                NotificationLifecycle::Transient => {
                    summary.notifications.tombstone(notification.id);
                }
            }
        }
        Ok(summary)
    }

    /// Creates a notification: the store assigns the id and timestamps it.
    /// This is the single door every side-effecting operation uses to create
    /// a notification (ADR-0010) — notification creation lives here, not in
    /// the calling entity.
    pub(crate) async fn create(
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
        store: &mut impl NotificationStore,
    ) -> TbResult<Notification> {
        store
            .notification_create(recipient, lifecycle, kind, subject, message)
            .await
    }

    /// All of a user's notifications as a plain list (no lifecycle applied).
    /// The session-less read used by the summary assembly (ADR-0010): the
    /// caller already owns the recipient's data, so no ownership check.
    pub(crate) async fn all_for_user(
        recipient: UserId,
        store: &mut impl NotificationStore,
    ) -> TbResult<Vec<Notification>> {
        store.notification_all_for_user(recipient).await
    }
}

#[cfg(test)]
mod notification_tests;
