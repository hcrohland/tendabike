# Garage Subscription System

## Overview

The Garage system in Tendabike allows users to create garages and manage bike maintenance for multiple users through a subscription-based model. Garage owners can accept subscription requests from other users, who can then register their bikes to the garage for maintenance tracking.

## Key Features

### 1. Garage Management
- **Create Garages**: Users can create their own garages with a name and description
- **View Garages**: Users see their owned garages with bike counts and management options
- **Edit/Delete Garages**: Full CRUD operations for garage owners
- **Search Garages**: Search by garage name or owner name (first name, last name, or full name)

### 2. Subscription System
- **Request Subscription**: Users can search for garages and request to subscribe
- **Approve/Reject Requests**: Garage owners can approve or reject subscription requests with optional messages
- **Active Subscriptions**: Approved users can register their bikes to the garage
- **Unsubscribe**: Users can cancel their subscriptions at any time
- **Status Management**: Track subscription status (pending, active, rejected, cancelled)

### 3. Bike Registration
- **Register Bikes**: Subscribed users can register their bikes to the garage
- **Unregister Bikes**: Garage owners can remove bikes from their garage
- **View Registered Bikes**: Garage owners can see all bikes registered to their garage

## User Interface

### Navigation Structure
```
Garages
├── My Subscriptions (default tab)
│   ├── Subscription List (with garage names and owner info)
│   └── Find Garages (search and request subscriptions)
└── My Garages
    ├── Your Garages (garage cards)
    └── Pending Subscription Requests (grouped by garage)
```

### Garage Cards
Each garage card displays:
- Garage name
- Owner name (for garages you don't own)
- Description
- Number of bikes (for owned garages)
- Owner badge (for owned garages)
- Creation date
- Action menu (Edit/Delete for owners, Request Subscription for others)

### Subscription Table
The subscription table shows:
- **Garage Column**: Garage name with owner info (e.g., "Bike Shop (John Doe)")
- **Request Message**: Optional message from subscriber
- **Response**: Optional message from garage owner
- **Status**: Color-coded badge (pending/active/rejected/cancelled)
- **Date**: When the subscription was created
- **Actions**: Context-specific actions based on status and role

## Technical Architecture

### Backend Structure

#### Domain Layer (`backend/domain/src/entities/garage.rs`)

**Core Entities:**
- `Garage`: Basic garage information (id, owner, name, description, created_at)
- `GarageWithOwner`: Garage with owner details (includes owner_firstname, owner_name)
- `GarageSubscription`: Subscription between user and garage
- `GarageSubscriptionWithDetails`: Subscription with full garage and owner information
- `SubscriptionStatus`: Enum (Pending, Active, Rejected, Cancelled)

**Key Methods:**
```rust
// Garage operations
GarageId::create()
GarageId::update()
GarageId::delete()
Garage::search()
Garage::with_owner_info()

// Part (bike) operations
GarageId::register_part()
GarageId::unregister_part()
GarageId::get_parts()

// Subscription operations
SubscriptionId::create()
SubscriptionId::approve()
SubscriptionId::reject()
SubscriptionId::cancel()
GarageSubscription::get_for_user()
GarageSubscription::get_pending_for_garage()
GarageSubscription::with_garage_details()
```

#### Database Layer (`backend/sqlx/src/store/garage.rs`)

**Tables:**
- `garages`: Main garage information
- `garage_subscriptions`: User subscriptions to garages
- `garage_parts`: Junction table linking bikes to garages

**Advanced Search Query:**
The search functionality supports multiple search patterns:
```sql
SELECT DISTINCT g.id, g.owner, g.name, g.description, g.created_at
FROM garages g
LEFT JOIN users u ON g.owner = u.id
WHERE g.name ILIKE $1
   OR COALESCE(u.firstname, '') ILIKE $1
   OR COALESCE(u.name, '') ILIKE $1
   OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
   OR CONCAT(g.name, ' ', COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
   OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, ''), ' ', g.name) ILIKE $1
```

This allows searching by:
- Garage name: "Bike Shop"
- Owner first name: "John"
- Owner last name: "Doe"
- Owner full name: "John Doe"
- Combined: "Bike Shop John", "John Bike Shop", etc.

#### API Layer (`backend/axum/src/domain/garage.rs`)

**Endpoints:**
```
GET    /api/garage              - List user's garages
POST   /api/garage              - Create a garage
GET    /api/garage/{id}         - Get garage details
PUT    /api/garage/{id}         - Update garage
DELETE /api/garage/{id}         - Delete garage
GET    /api/garage/search?q=    - Search garages
GET    /api/garage/{id}/parts   - Get garage bikes

POST   /api/garage/{id}/parts/{part_id}   - Register bike
DELETE /api/garage/{id}/parts/{part_id}   - Unregister bike

POST   /api/garage/subscriptions                    - Request subscription
GET    /api/garage/subscriptions                    - List my subscriptions (with garage details)
GET    /api/garage/{id}/subscriptions               - List pending requests for garage
POST   /api/garage/subscriptions/{id}/approve       - Approve subscription
POST   /api/garage/subscriptions/{id}/reject        - Reject subscription
DELETE /api/garage/subscriptions/{id}               - Cancel/delete subscription
```

### Frontend Structure

#### Components

**`Garages.svelte`** - Main container
- Manages tab navigation
- Displays "My Subscriptions" and "My Garages" tabs

**`GarageList.svelte`** - Garage grid display
- Renders garage cards in a responsive grid
- Passes isOwner flag to determine available actions

**`GarageCard.svelte`** - Individual garage card
- Displays garage information
- Shows owner name for non-owned garages
- Provides action menu (Edit/Delete/Subscribe)
- Loads and displays bike count for owned garages

**`Subscriptions.svelte`** - Subscription management
- Lists subscriptions in a table format
- Handles subscription actions (approve/reject/unsubscribe/delete)
- Includes garage search functionality
- Shows tooltips for long messages
- Implements inline confirmation dialogs

**`SubscriptionRequestModal.svelte`** - Subscription request form
- Allows users to add optional message when requesting subscription
- Dispatches custom event on success for real-time updates

#### State Management

**Stores:**
- `garages`: Mapable store of user's owned garages (from summary endpoint)
- Garage data includes owner information (owner_firstname, owner_name)

**Event System:**
```javascript
// Dispatched when subscription is created
window.dispatchEvent(new CustomEvent('subscription-updated'))

// Listened to by Subscriptions component to refresh data
window.addEventListener('subscription-updated', handleSubscriptionUpdate)
```

#### Data Models

**`Garage` class** (`frontend/src/lib/garage.ts`):
```typescript
class Garage {
  id?: number;
  owner: number;
  owner_firstname: string;    // Owner's first name
  owner_name: string;          // Owner's last name
  name: string;
  description?: string;
  created_at: Date;
}
```

**`GarageSubscription` interface**:
```typescript
interface GarageSubscription {
  id: number;
  garage_id: number;
  garage_name?: string;               // Included in "My Subscriptions"
  garage_owner_firstname?: string;    // Included in "My Subscriptions"
  garage_owner_name?: string;         // Included in "My Subscriptions"
  user_id: number;
  status: "pending" | "active" | "rejected" | "cancelled";
  message?: string;
  response_message?: string;
  created_at: string;
  updated_at: string;
}
```

## Permission Model

### Garage Owners Can:
- Create, edit, and delete their garages
- View all bikes registered to their garage
- Approve or reject subscription requests
- Unregister any bike from their garage
- View all subscription requests for their garages

### Subscribers Can:
- Search for garages
- Request subscriptions to any garage
- Register their own bikes to garages they have active subscriptions to
- Unregister their own bikes
- Cancel their subscriptions (pending, active, or rejected)

### Access Rules:
- Users can only see garages they own or have subscriptions to
- Bike registration requires either:
  - Being the garage owner, OR
  - Having an active subscription to the garage
- Only garage owners can approve/reject subscription requests
- Only subscribers can cancel their subscriptions

## User Workflows

### Creating and Managing a Garage

1. **Create a Garage:**
   - Click "Create Garage" button
   - Fill in garage name and optional description
   - Submit to create

2. **Manage Subscription Requests:**
   - Navigate to "My Garages" tab
   - View pending requests under each garage
   - Click "Respond" to approve or reject with optional message

3. **View Registered Bikes:**
   - Garage card shows bike count
   - Click garage for details

### Subscribing to a Garage

1. **Find a Garage:**
   - Go to "My Subscriptions" tab
   - Scroll to "Find Garages" section
   - Search by garage name or owner name
   - Search supports partial matches and combinations

2. **Request Subscription:**
   - Click "Request Subscription" from garage card menu
   - Add optional message explaining why you want to subscribe
   - Submit request

3. **Wait for Approval:**
   - Request appears in "My Subscriptions" with "pending" status
   - Owner receives notification in their "Pending Subscription Requests"

4. **Register Bikes (after approval):**
   - Once approved, status changes to "active"
   - You can now register your bikes to the garage

### Managing Subscriptions

1. **View Your Subscriptions:**
   - "My Subscriptions" tab shows all your subscriptions
   - Each row displays:
     - Garage name with owner: "Bike Shop (John Doe)"
     - Your request message
     - Owner's response (if any)
     - Current status

2. **Handle Different Statuses:**
   - **Pending**: Wait for owner response or cancel request
   - **Active**: Can register bikes, can unsubscribe
   - **Rejected**: Can delete the rejected request, owner's reason visible
   - **Cancelled**: No longer have access

3. **Unsubscribe:**
   - Click "Unsubscribe" for active subscriptions
   - Confirm in inline dialog
   - Your bikes will need to be unregistered first

## UI Features

### Search Functionality
- **Debounced Input**: 300ms delay to avoid excessive API calls
- **Partial Matching**: Matches partial strings in any field
- **Case Insensitive**: Uses ILIKE in database
- **Combined Search**: Can search "Shop John" to find "Bike Shop" owned by "John"

### Message Display
- **Truncation**: Messages longer than 50 characters are truncated
- **Tooltips**: Hover over truncated messages to see full text
- **Optional Messages**: Both request and response messages are optional

### Inline Confirmations
- **Unsubscribe**: Confirms before unsubscribing from active subscriptions
- **Delete**: Confirms before deleting rejected subscriptions
- **No Modal**: Confirmation UI appears inline in the table

### Real-time Updates
- **Custom Events**: Components communicate via window events
- **Automatic Refresh**: Subscription list updates when new requests are made
- **Optimistic UI**: Updates shown immediately with loading states

## Data Flow Examples

### Creating a Subscription Request

**Frontend:**
```typescript
// User clicks "Request Subscription" in GarageCard
// Opens SubscriptionRequestModal
// User enters optional message and submits

await garage.requestSubscription(message);
window.dispatchEvent(new CustomEvent('subscription-updated'));
```

**Backend:**
```rust
// POST /api/garage/subscriptions
// Create subscription with status: pending
SubscriptionId::create(garage_id, message, user, store)
```

**Result:** Subscription appears in both:
- Subscriber's "My Subscriptions" list (pending status)
- Owner's "Pending Subscription Requests" section

### Approving a Subscription

**Frontend:**
```typescript
// Garage owner clicks "Respond" → "Approve"
// Enters optional response message
// Submits

await myfetch(`/api/garage/subscriptions/${id}/approve`, "POST", {
  message: responseMessage
});
```

**Backend:**
```rust
// POST /api/garage/subscriptions/{id}/approve
// Updates status to active, stores response message
subscription_id.approve(response_message, user, store)
```

**Result:**
- Status changes to "active"
- Subscriber can now register bikes
- Response message visible to subscriber

### Searching for Garages

**Frontend:**
```typescript
// User types in search box
// Debounced function calls API after 300ms

const results = await myfetch(
  `/api/garage/search?q=${encodeURIComponent(searchQuery)}`,
  "GET"
);
```

**Backend:**
```rust
// GET /api/garage/search?q=john+bike
// Searches in multiple fields with ILIKE
// Returns Vec<GarageWithOwner> with owner info
Garage::search(query, store)
  .then(|garages| Garage::with_owner_info(garages, store))
```

**Result:** Display garage cards with owner information

## Testing Checklist

### Garage CRUD
- [ ] Create garage with name and description
- [ ] Edit garage details
- [ ] Delete empty garage
- [ ] Cannot delete garage with registered bikes
- [ ] View garage list with correct bike counts

### Search Functionality
- [ ] Search by garage name
- [ ] Search by owner first name
- [ ] Search by owner last name
- [ ] Search by owner full name
- [ ] Search by combination (garage + owner)
- [ ] Partial matching works
- [ ] Case insensitive search
- [ ] Empty search shows no results

### Subscription Workflow
- [ ] Request subscription with message
- [ ] Request subscription without message
- [ ] Request appears in owner's pending list
- [ ] Request appears in subscriber's list
- [ ] Approve with response message
- [ ] Reject with response message
- [ ] Subscriber sees response messages
- [ ] Unsubscribe from active subscription
- [ ] Delete rejected subscription
- [ ] Cannot request duplicate subscriptions

### Bike Registration
- [ ] Register bike to owned garage
- [ ] Register bike with active subscription
- [ ] Cannot register without subscription
- [ ] Unregister bike as owner
- [ ] Bike count updates correctly

### UI/UX
- [ ] Owner name displays in garage cards
- [ ] Owner name displays in subscription list
- [ ] Long messages show tooltips
- [ ] Confirmation dialogs work
- [ ] Status badges have correct colors
- [ ] Real-time updates work
- [ ] Search debouncing works
- [ ] Tab navigation works

## Future Enhancements

### Potential Features
1. **Notifications**: Real-time notifications for subscription approvals/rejections
2. **Batch Operations**: Approve/reject multiple subscriptions at once
3. **Subscription Limits**: Set maximum number of subscribers per garage
4. **Subscription Expiry**: Auto-expire subscriptions after certain period
5. **User Profiles**: Show user avatars and additional info
6. **Garage Categories**: Categorize garages by type (personal, business, club)
7. **Reviews**: Allow subscribers to review garages
8. **Invitation System**: Invite users via email
9. **Advanced Permissions**: Set different permission levels for subscribers
10. **Activity Log**: Track all garage activities and changes

### Technical Improvements
1. **Pagination**: For large lists of garages/subscriptions
2. **Caching**: Cache garage and user data to reduce database queries
3. **Webhooks**: Notify external systems of subscription events
4. **Analytics**: Track garage usage and subscription metrics
5. **Export**: Export garage data and subscription history
6. **Bulk Import**: Import bikes in bulk
7. **API Rate Limiting**: Prevent abuse of search endpoint

## Troubleshooting

### Common Issues

**"Garage #1" showing instead of garage name:**
- Backend should return `GarageWithOwner` from endpoints
- Frontend should use subscription.garage_name when available
- Check that /api/garage/subscriptions returns full details

**Search not finding garages:**
- Ensure database has been restarted after search query changes
- Check that COALESCE is handling NULL values
- Verify LEFT JOIN is working correctly

**Permission denied errors:**
- Check user owns garage or has active subscription
- Verify garage owner for approval/rejection
- Ensure correct user ID in subscription

**Real-time updates not working:**
- Check that 'subscription-updated' event is dispatched
- Verify event listeners are attached in onMount
- Ensure event listeners are cleaned up in onDestroy

## Development Notes

### Database Migrations
Location: `backend/sqlx/migrations/`
- `20250202000000_add_garage_subscriptions.up.sql`: Initial subscription table
- `20250202000001_add_subscription_response_message.up.sql`: Response messages

### Key Files
**Backend:**
- `backend/domain/src/entities/garage.rs`: Core domain logic
- `backend/domain/src/traits/garage.rs`: Store traits
- `backend/sqlx/src/store/garage.rs`: Database queries
- `backend/axum/src/domain/garage.rs`: API endpoints

**Frontend:**
- `frontend/src/Garage/Garages.svelte`: Main container
- `frontend/src/Garage/GarageList.svelte`: Grid display
- `frontend/src/Garage/GarageCard.svelte`: Card component
- `frontend/src/Garage/Subscriptions.svelte`: Subscription management
- `frontend/src/lib/garage.ts`: Garage class and store

### Code Conventions
- Async methods use Result types with TbResult
- Frontend uses Svelte 5 runes ($state, $derived, $effect)
- Custom events for cross-component communication
- Inline confirmations instead of modal dialogs
- Optimistic UI updates with loading states
