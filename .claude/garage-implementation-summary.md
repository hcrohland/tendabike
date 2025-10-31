# Garage Management System Implementation Summary

## Overview

A complete garage management system that allows garage owners to manage multiple bikes and users to request registration of their bikes to garages for delegated maintenance tracking.

## Features

- **Garage CRUD**: Create, read, update, delete garages
- **Search**: Find garages by name (case-insensitive)
- **Request/Approval Workflow**: Users request bike registration, garage owners approve/reject
- **Part Management**: Register/unregister bikes to garages
- **Authorization**: Proper access control throughout
- **Duplicate Prevention**: Database constraints prevent duplicate pending requests

## Database Schema

### Tables

**garages**
```sql
CREATE TABLE IF NOT EXISTS garages (
    id SERIAL PRIMARY KEY,
    owner INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**garage_parts** (Junction Table)
```sql
CREATE TABLE IF NOT EXISTS garage_parts (
    garage_id INTEGER NOT NULL REFERENCES garages(id) ON DELETE CASCADE,
    part_id INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (garage_id, part_id)
);
```

**garage_registration_requests**
```sql
CREATE TABLE IF NOT EXISTS garage_registration_requests (
    id SERIAL PRIMARY KEY,
    garage_id INTEGER NOT NULL REFERENCES garages(id) ON DELETE CASCADE,
    part_id INTEGER NOT NULL REFERENCES parts(id) ON DELETE CASCADE,
    requester_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(garage_id, part_id, status)
);
```

### Migrations

- `20250131000000_add_garages.up.sql` - Core garage infrastructure
- `20250131000001_add_garage_registration_requests.up.sql` - Request/approval workflow

## Backend Architecture

### Domain Layer

**File**: `backend/domain/src/entities/garage.rs`

**Key Types**:
```rust
pub struct Garage {
    pub id: GarageId,
    pub owner: UserId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: OffsetDateTime,
}

pub struct GarageRegistrationRequest {
    pub id: RegistrationRequestId,
    pub garage_id: GarageId,
    pub part_id: PartId,
    pub requester_id: UserId,
    pub status: RegistrationRequestStatus,
    pub message: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub enum RegistrationRequestStatus {
    Pending,
    Approved,
    Rejected,
}
```

**Key Methods**:

*GarageId*:
- `get()` - Get garage by ID with authorization
- `create()` - Create new garage
- `update()` - Update garage details
- `delete()` - Delete garage (only if no bikes)
- `register_part()` - Register bike to garage
- `unregister_part()` - Remove bike from garage
- `get_parts()` - List all bikes in garage
- `read()` - Get garage details
- `checkuser()` - Verify ownership

*RegistrationRequestId*:
- `create()` - Create registration request
- `get()` - Get request by ID with authorization
- `read()` - Get request details
- `approve()` - Approve request (garage owner only)
- `reject()` - Reject request (garage owner only)
- `cancel()` - Cancel request (requester only)
- `checkuser()` - Verify access (requester or garage owner)

*Garage*:
- `get_all_for_user()` - List user's garages
- `search()` - Search garages by name

*GarageRegistrationRequest*:
- `get_pending_for_garage()` - List pending requests for garage
- `get_for_user()` - List user's requests

### Traits Layer

**File**: `backend/domain/src/traits/garage.rs`

```rust
#[async_trait::async_trait]
pub trait GarageStore {
    // Garage CRUD
    async fn garage_create(...) -> TbResult<Garage>;
    async fn garage_get(&mut self, id: GarageId) -> TbResult<Garage>;
    async fn garage_update(...) -> TbResult<Garage>;
    async fn garage_delete(&mut self, id: GarageId) -> TbResult<usize>;
    async fn garages_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Garage>>;
    async fn garages_search(&mut self, query: &str) -> TbResult<Vec<Garage>>;

    // Part registration
    async fn garage_register_part(...) -> TbResult<()>;
    async fn garage_unregister_part(...) -> TbResult<()>;
    async fn garage_get_parts(&mut self, garage_id: GarageId) -> TbResult<Vec<PartId>>;
    async fn part_get_garage(&mut self, part_id: PartId) -> TbResult<Option<GarageId>>;

    // Registration requests
    async fn registration_request_create(...) -> TbResult<GarageRegistrationRequest>;
    async fn registration_request_get(&mut self, id: RegistrationRequestId) -> TbResult<GarageRegistrationRequest>;
    async fn registration_request_find_pending(...) -> TbResult<Option<GarageRegistrationRequest>>;
    async fn registration_request_update_status(...) -> TbResult<GarageRegistrationRequest>;
    async fn registration_request_delete(&mut self, id: RegistrationRequestId) -> TbResult<()>;
    async fn registration_requests_for_garage(...) -> TbResult<Vec<GarageRegistrationRequest>>;
    async fn registration_requests_for_user(&mut self, user_id: UserId) -> TbResult<Vec<GarageRegistrationRequest>>;
}
```

### Persistence Layer

**File**: `backend/sqlx/src/store/garage.rs`

Complete PostgreSQL implementation of all GarageStore methods using sqlx with prepared queries.

**Key Implementation Details**:
- Type-safe queries with `sqlx::query_as!`
- Proper error handling with `into_domain`
- Case-insensitive search with ILIKE
- Conflict handling for duplicate registrations
- Automatic timestamp updates via database triggers

### HTTP Layer

**File**: `backend/axum/src/domain/garage.rs`

**REST API Endpoints**:

*Garage Management*:
- `GET /api/garage/` - List user's garages
- `POST /api/garage/` - Create garage
- `GET /api/garage/search?q=query` - Search garages
- `GET /api/garage/{id}` - Get garage details
- `PUT /api/garage/{id}` - Update garage
- `DELETE /api/garage/{id}` - Delete garage (only if no bikes)

*Part Registration*:
- `GET /api/garage/{id}/parts` - List bikes in garage
- `POST /api/garage/{id}/parts/{part_id}` - Register bike to garage
- `DELETE /api/garage/{id}/parts/{part_id}` - Unregister bike from garage

*Registration Requests*:
- `GET /api/garage/requests` - List user's registration requests
- `POST /api/garage/requests` - Create registration request
- `GET /api/garage/requests/{id}` - Get request details
- `DELETE /api/garage/requests/{id}` - Cancel request (requester only)
- `POST /api/garage/requests/{id}/approve` - Approve request (garage owner only)
- `POST /api/garage/requests/{id}/reject` - Reject request (garage owner only)
- `GET /api/garage/{id}/requests` - List pending requests for garage

**Request/Response DTOs**:
```rust
pub struct NewGarage {
    pub name: String,
    pub description: Option<String>,
}

pub struct UpdateGarage {
    pub name: String,
    pub description: Option<String>,
}

pub struct NewRegistrationRequest {
    pub garage_id: i32,
    pub part_id: i32,
    pub message: Option<String>,
}
```

## Frontend Integration

**File**: `frontend/src/lib/garage.ts`

```typescript
export class Garage {
  id?: number;
  owner: number;
  name: string;
  description?: string;
  created_at: Date;

  async create(): Promise<Garage>
  async update(): Promise<void>
  async delete(): Promise<void>
  async registerPart(partId: number): Promise<void>
  async unregisterPart(partId: number): Promise<void>
  async getParts(): Promise<number[]>
}

export const garages = mapable<Garage>("garage", Garage);
```

**Store Integration** (`frontend/src/lib/store.ts`):
- Garages added to Summary type
- Auto-fetched and updated with user summary
- Reactive store for Svelte components

## Authorization Model

**Access Control Rules**:

1. **Garage Operations**:
   - Only garage owner can update/delete garage
   - Only garage owner can register/unregister bikes directly
   - Anyone can search and view garage details (for discovery)

2. **Registration Requests**:
   - Any user can create request for bikes they own
   - Only requester can cancel pending requests
   - Only garage owner can approve/reject requests
   - Both requester and garage owner can view request details

3. **Authorization Implementation**:
   - `checkuser()` methods verify ownership
   - `Person::check_owner()` enforces access control
   - Database-level CASCADE deletes for data consistency

## Business Logic Rules

1. **Garage Deletion**: Only allowed if garage has no registered bikes
2. **Duplicate Requests**: Database constraint prevents multiple pending requests for same bike+garage
3. **Already Registered**: Cannot create request if bike already registered to garage
4. **Request State**: Can only approve/reject pending requests
5. **Auto-Registration**: Approving request automatically registers bike to garage
6. **Cascade Deletion**: Deleting garage removes all associated parts and requests

## Testing Checklist

- [ ] Create garage as user
- [ ] Search for garages
- [ ] Create registration request for owned bike
- [ ] Approve request as garage owner
- [ ] Verify bike appears in garage parts list
- [ ] Reject registration request
- [ ] Cancel pending request as requester
- [ ] Try to delete garage with bikes (should fail)
- [ ] Unregister bike from garage
- [ ] Delete empty garage
- [ ] Test authorization (try accessing other users' resources)
- [ ] Test duplicate request prevention

## Database Setup

```bash
cd backend/sqlx
sqlx migrate run
cargo sqlx prepare
```

## Compilation Status

✅ All code compiles successfully
✅ Database migrations created
✅ sqlx offline metadata generated
✅ No warnings or errors

## Future Enhancements (Not Implemented)

- Frontend UI components for garage management
- Notification system for request status changes
- Pagination for search results (currently limited to 50)
- Garage stats/metrics dashboard
- Bulk bike registration
- Request message history/conversation
- Email notifications for requests
