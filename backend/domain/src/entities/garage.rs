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

//! This module contains the tb_domain logic for garages in the Tendabike system.
//!
//! A `Garage` represents a collection of bikes owned by a garage owner. Users can register
//! their bikes to a garage, delegating maintenance to the garage owner.

use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::*;

/// The database's representation of a garage.
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Garage {
    /// The primary key
    pub id: GarageId,
    /// The owner of the garage
    pub owner: UserId,
    /// The name of the garage
    pub name: String,
    /// Optional description of the garage
    pub description: Option<String>,
    /// Creation timestamp
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GarageId(i32);

NewtypeDisplay! { () pub struct GarageId(); }
NewtypeFrom! { () pub struct GarageId(i32); }

impl GarageId {
    /// Get a garage by ID, checking that the user has access to it
    pub async fn get(
        id: i32,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<GarageId> {
        GarageId(id).checkuser(user, store).await
    }

    /// Check if the user owns this garage or is an admin
    pub async fn checkuser(
        self,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<GarageId> {
        let garage = store.garage_get(self).await?;
        user.check_owner(garage.owner, "Access denied to garage".to_string())?;
        Ok(self)
    }

    /// Create a new garage
    pub async fn create(
        name: String,
        description: Option<String>,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<Garage> {
        store.garage_create(name, description, user.get_id()).await
    }

    /// Update an existing garage
    pub async fn update(
        self,
        name: String,
        description: Option<String>,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<Garage> {
        self.checkuser(user, store).await?;
        store.garage_update(self, name, description).await
    }

    /// Delete a garage (only if it has no bikes)
    pub async fn delete(self, user: &dyn Person, store: &mut impl Store) -> TbResult<GarageId> {
        self.checkuser(user, store).await?;

        // Check if garage has any bikes
        let parts = store.garage_get_parts(self).await?;
        if !parts.is_empty() {
            return Err(Error::Conflict("Garage still has bikes assigned".into()));
        }

        store.garage_delete(self).await?;
        Ok(self)
    }

    /// Register a part (bike) to this garage
    pub async fn register_part(
        self,
        part_id: PartId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<()> {
        self.checkuser(user, store).await?;

        // Verify the part exists and user has access to it
        part_id.checkuser(user, store).await?;

        store.garage_register_part(self, part_id).await
    }

    /// Unregister a part (bike) from this garage
    pub async fn unregister_part(
        self,
        part_id: PartId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<()> {
        self.checkuser(user, store).await?;
        store.garage_unregister_part(self, part_id).await
    }

    /// Get all parts registered to this garage
    pub async fn get_parts(
        self,
        user: &dyn Person,
        store: &mut impl GarageStore,
    ) -> TbResult<Vec<PartId>> {
        self.checkuser(user, store).await?;
        store.garage_get_parts(self).await
    }

    /// Read a garage from the database
    pub async fn read(self, user: &dyn Person, store: &mut impl GarageStore) -> TbResult<Garage> {
        self.checkuser(user, store).await?;
        store.garage_get(self).await
    }
}

impl Garage {
    /// Get all garages for a user
    pub async fn get_all_for_user(
        user: &UserId,
        store: &mut impl GarageStore,
    ) -> TbResult<Vec<Garage>> {
        store.garages_get_all_for_user(*user).await
    }

    /// Search for garages by name (for users to find garages to request registration)
    pub async fn search(query: &str, store: &mut impl GarageStore) -> TbResult<Vec<Garage>> {
        store.garages_search(query).await
    }
}

/// Registration request status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RegistrationRequestStatus {
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for RegistrationRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationRequestStatus::Pending => write!(f, "pending"),
            RegistrationRequestStatus::Approved => write!(f, "approved"),
            RegistrationRequestStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// A request to register a bike to a garage
#[serde_as]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GarageRegistrationRequest {
    pub id: RegistrationRequestId,
    pub garage_id: GarageId,
    pub part_id: PartId,
    pub requester_id: UserId,
    pub status: RegistrationRequestStatus,
    pub message: Option<String>,
    #[serde_as(as = "Rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde_as(as = "Rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRequestId(i32);

NewtypeDisplay! { () pub struct RegistrationRequestId(); }
NewtypeFrom! { () pub struct RegistrationRequestId(i32); }

impl RegistrationRequestId {
    /// Create a new registration request
    pub async fn create(
        garage_id: GarageId,
        part_id: PartId,
        message: Option<String>,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageRegistrationRequest> {
        // Verify the garage exists
        garage_id.read(user, store).await?;

        // Verify the user owns the part
        part_id.checkuser(user, store).await?;

        // Check if there's already a pending request
        let existing = store
            .registration_request_find_pending(garage_id, part_id)
            .await?;
        if existing.is_some() {
            return Err(Error::Conflict(
                "A pending request already exists for this bike".into(),
            ));
        }

        // Check if the part is already registered to this garage
        let garage_parts = store.garage_get_parts(garage_id).await?;
        if garage_parts.contains(&part_id) {
            return Err(Error::Conflict(
                "This bike is already registered to the garage".into(),
            ));
        }

        store
            .registration_request_create(garage_id, part_id, user.get_id(), message)
            .await
    }

    /// Get a registration request by ID
    pub async fn get(
        id: i32,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<RegistrationRequestId> {
        RegistrationRequestId(id).checkuser(user, store).await
    }

    /// Read a registration request from the database
    pub async fn read(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageRegistrationRequest> {
        self.checkuser(user, store).await?;
        store.registration_request_get(self).await
    }

    /// Check if the user has access to this request (either requester or garage owner)
    pub async fn checkuser(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<RegistrationRequestId> {
        let request = store.registration_request_get(self).await?;

        // Allow access if user is the requester
        if request.requester_id == user.get_id() {
            return Ok(self);
        }

        // Allow access if user owns the garage
        let garage = store.garage_get(request.garage_id).await?;
        user.check_owner(
            garage.owner,
            "Access denied to registration request".to_string(),
        )?;

        Ok(self)
    }

    /// Approve a registration request (garage owner only)
    pub async fn approve(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageRegistrationRequest> {
        let request = store.registration_request_get(self).await?;

        // Verify user owns the garage
        let garage_id = request.garage_id;
        garage_id.checkuser(user, store).await?;

        if request.status != RegistrationRequestStatus::Pending {
            return Err(Error::Conflict("Request is not pending".into()));
        }

        // Register the part to the garage
        store
            .garage_register_part(request.garage_id, request.part_id)
            .await?;

        // Update request status
        store
            .registration_request_update_status(self, RegistrationRequestStatus::Approved)
            .await
    }

    /// Reject a registration request (garage owner only)
    pub async fn reject(
        self,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<GarageRegistrationRequest> {
        let request = store.registration_request_get(self).await?;

        // Verify user owns the garage
        let garage_id = request.garage_id;
        garage_id.checkuser(user, store).await?;

        if request.status != RegistrationRequestStatus::Pending {
            return Err(Error::Conflict("Request is not pending".into()));
        }

        store
            .registration_request_update_status(self, RegistrationRequestStatus::Rejected)
            .await
    }

    /// Cancel a registration request (requester only)
    pub async fn cancel(self, user: &dyn Person, store: &mut impl Store) -> TbResult<()> {
        let request = store.registration_request_get(self).await?;

        // Verify user is the requester
        user.check_owner(
            request.requester_id,
            "Access denied - not the requester".to_string(),
        )?;

        if request.status != RegistrationRequestStatus::Pending {
            return Err(Error::Conflict("Can only cancel pending requests".into()));
        }

        store.registration_request_delete(self).await
    }
}

impl GarageRegistrationRequest {
    /// Get all pending requests for a garage (garage owner only)
    pub async fn get_pending_for_garage(
        garage_id: GarageId,
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageRegistrationRequest>> {
        garage_id.checkuser(user, store).await?;
        store
            .registration_requests_for_garage(garage_id, Some(RegistrationRequestStatus::Pending))
            .await
    }

    /// Get all requests made by a user
    pub async fn get_for_user(
        user: &dyn Person,
        store: &mut impl Store,
    ) -> TbResult<Vec<GarageRegistrationRequest>> {
        store.registration_requests_for_user(user.get_id()).await
    }
}
