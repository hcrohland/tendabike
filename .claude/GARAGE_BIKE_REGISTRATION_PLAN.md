# Garage Bike and Spare Parts Registration - Implementation Plan

## Overview

This document outlines the plan to implement a UI for registering bikes and spare parts to garages. The backend API already supports part registration (both bikes and spare parts), but the frontend needs UI components to manage this functionality.

**Key Distinction**:
- **Bikes (Gear)**: Parts where `part.isGear()` returns `true` - these are main items like road bikes, mountain bikes, etc.
- **Spare Parts**: Parts where `part.isGear()` returns `false` - these are components like wheels, chains, cassettes, etc. that can be attached to bikes

## Current Backend Implementation

### API Endpoints (Already Implemented)

From [backend/axum/src/domain/garage.rs](backend/axum/src/domain/garage.rs:72-76):

```
POST   /api/garage/{garage_id}/parts        - Register a bike (body: { part_id: number })
DELETE /api/garage/{garage_id}/parts/{part_id} - Unregister a bike
GET    /api/garage/{garage_id}/parts        - Get all bikes registered to garage
```

### Permission Logic

From [backend/domain/src/entities/garage.rs](backend/domain/src/entities/garage.rs:152-180):

**Registration (`register_part`)**:
- User must own the part (bike or spare part) being registered
- User must be EITHER:
  - Garage owner, OR
  - Have an active subscription to the garage
- Error message: "You must subscribe to this garage before registering bikes"
- Applies to both bikes (gear) and spare parts

**Unregistration (`unregister_part`)**:
- Garage owners can unregister any parts (bikes or spare parts)
- Part owners can unregister their own parts
- Verified via `checkuser()` which ensures garage owner or admin
- **TODO**: Backend needs update to allow part owners to unregister their own parts

**View Parts (`get_parts`)**:
- Returns `Vec<PartId>` (just IDs, not full Part objects)
- Need to fetch full Part details separately

### Frontend Models

**Part Model** ([frontend/src/lib/part.ts](frontend/src/lib/part.ts)):
```typescript
class Part {
  id?: number;
  owner: number;
  what: number;          // Type ID - determines if it's gear or spare part
  name: string;
  vendor: string;
  model: string;
  purchase: Date;
  last_used: Date;
  disposed_at?: Date;
  usage: string;

  // Helper methods
  isGear(): boolean      // Returns true if this is a bike (main gear), false if spare part
  type(): Type           // Returns the type object with metadata
}
```

**Garage Model** ([frontend/src/lib/garage.ts](frontend/src/lib/garage.ts:50-72)):
```typescript
class Garage {
  async registerPart(partId: number)    // Already implemented
  async unregisterPart(partId: number)  // Already implemented
  async getParts(): Promise<number[]>   // Already implemented - returns IDs only
}
```

## UI Requirements

### 1. Garage Detail View with Parts List

**Location**: New component `frontend/src/Garage/GarageDetail.svelte`

**Purpose**: Show full details of a garage including all registered bikes and spare parts

**Access**:
- Garage owners can see all parts registered to their garage with full owner information
- Subscribers with active subscriptions can see ONLY their own parts (not other users' parts)

**Features**:
- Display garage information (name, description, owner)
- **Tabbed view** (confirmed approach):
  - **Bikes (Gear)** tab: Show registered bikes
  - **Spare Parts** tab: Show registered spare parts
- List parts based on user role:
  - **Garage owners see**: ALL parts with owner names, models, type badges, usage
  - **Subscribers see**: ONLY their own parts with models, type badges, usage
  - Part name and model
  - Part type badge (e.g., "Road Bike", "Chain", "Wheel")
  - Owner name (only for garage owners)
  - Current usage/mileage (for bikes)
  - Last service date (if available)
- Action menu for each part:
  - **Garage owners**: Unregister any part, View part details
  - **Part owners**: Unregister own part, View part details
  - **Other subscribers**: View part details only
- Button to register new bikes/parts (if user has permission)

**UI Layout** (Tabbed - Confirmed Approach):
```
┌─────────────────────────────────────────┐
│ [Back] Garage Name                      │
│ by Owner Name                           │
│ Description                             │
│                                         │
│ [Register Bikes/Parts]  (if subscriber) │
│                                         │
│ [Bikes (5)] [Spare Parts (12)]          │
│                                         │
│ Registered Bikes                        │
│ ┌─────────────────────────────────────┐ │
│ │ 🚴 Trek Road Bike (John Doe)   [⋮] │ │
│ │    Model: Domane SL 5               │ │
│ │    Usage: 2,450 km                  │ │
│ └─────────────────────────────────────┘ │
│ ┌─────────────────────────────────────┐ │
│ │ 🚵 Giant MTB (Jane Smith)      [⋮] │ │
│ │    Model: Trance X 29               │ │
│ │    Usage: 1,820 km                  │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Note**: Subscribers only see their own parts in the list, not other users' parts.

### 2. Part Registration Modal

**Location**: New component `frontend/src/Garage/RegisterPartModal.svelte` (renamed from RegisterBikeModal)

**Purpose**: Allow subscribers to register their bikes and spare parts to a garage

**Features**:
- Modal/dialog that opens when clicking "Register Bikes/Parts"
- Two tabs or sections:
  - **Bikes**: List user's bikes (where `part.isGear()` returns true)
  - **Spare Parts**: List user's spare parts (where `part.isGear()` returns false)
- Filter out:
  - Parts already registered to this garage
  - Disposed parts (`disposed_at != null`)
- **Note**: Parts can be registered to multiple garages (confirmed - no constraint)
- Show part info: name, model, type
- Radio button or checkbox selection
- Confirm button to register selected part(s)

**UI Layout**:
```
┌────────────────────────────────────┐
│ Register to "Garage Name"          │
│                                    │
│ [Bikes (3)] [Spare Parts (8)]      │
│                                    │
│ Select bikes to register:          │
│                                    │
│ ☐ Trek Road Bike                   │
│   Domane SL 5 • Road Bike          │
│                                    │
│ ☐ Giant Mountain Bike              │
│   Trance X 29 • Mountain Bike      │
│                                    │
│ ☐ Specialized Gravel               │
│   Diverge • Gravel Bike            │
│                                    │
│     [Cancel] [Register Selected]   │
└────────────────────────────────────┘

(Switch to Spare Parts tab)

┌────────────────────────────────────┐
│ Register to "Garage Name"          │
│                                    │
│ [Bikes (3)] [Spare Parts (8)]      │
│                                    │
│ Select spare parts to register:    │
│                                    │
│ ☐ Shimano Chain                    │
│   HG701 • Chain                    │
│                                    │
│ ☐ Continental Tire                 │
│   GP5000 • Tire                    │
│                                    │
│ ☐ DT Swiss Wheel                   │
│   R470 • Wheel                     │
│                                    │
│     [Cancel] [Register Selected]   │
└────────────────────────────────────┘
```

### 3. Enhanced Garage Card

**Location**: Update [frontend/src/Garage/GarageCard.svelte](frontend/src/Garage/GarageCard.svelte)

**Changes**:
- Add "View Details" action to menu
- Navigate to new GarageDetail view
- Update badge to show total parts count or separate counts:
  - Option A: "5 bikes" → "17 items" (bikes + spare parts combined)
  - Option B: "5 bikes, 12 parts" (separate counts)
  - Option C: Keep "5 bikes" for simplicity in card view

### 4. Part Registration from Part View

**Location**: Update [frontend/src/Part/Part.svelte](frontend/src/Part/Part.svelte)

**Purpose**: Allow users to register bikes AND spare parts to garages from the part detail page

**Features**:
- Add new menu item "Register to Garage"
- Available for BOTH bikes (gear) and spare parts
- Opens modal/dropdown showing user's subscriptions
- Lists only garages where user has active subscription or is owner
- Confirm to register

**UI Layout**:
```
Part Menu:
├── Attach (for spare parts only)
├── Dispose
├── Change details
├── Register to Garage  ← NEW (for all parts)
│   └── Opens modal with garage list
└── Delete
```

**Note**: The "Register to Garage" option should be available for:
- Bikes (gear) where `part.isGear()` returns true
- Spare parts where `part.isGear()` returns false
- Both types can be registered to garages for maintenance tracking

### 5. Garage Management in "My Garages" Tab

**Location**: Update [frontend/src/Garage/Garages.svelte](frontend/src/Garage/Garages.svelte)

**Changes**:
- Add "View Details" button to each garage card
- Shows part count (bikes + spare parts, already implemented)
- Navigate to GarageDetail view on click

## Component Architecture

```
Garages.svelte (Main container)
├── TabItem: My Subscriptions
│   ├── Subscriptions.svelte
│   └── GarageSearch (existing)
└── TabItem: My Garages
    ├── GarageList.svelte
    │   └── GarageCard.svelte
    │       └── [View Details] → GarageDetail.svelte ← NEW
    │
    └── Pending Subscription Requests

GarageDetail.svelte ← NEW COMPONENT
├── Garage header info
├── RegisterPartModal.svelte ← NEW COMPONENT (conditionally shown)
├── Tabs or Filters:
│   ├── All Items
│   ├── Bikes Only
│   └── Spare Parts Only
└── RegisteredPartList ← inline or separate component
    └── RegisteredPartCard × N ← inline or separate component

Part.svelte (Part detail view - bikes AND spare parts)
└── Menu
    └── RegisterToGarageModal.svelte ← NEW COMPONENT
```

## Data Flow

### Registering Parts to Garage

**From Garage View**:
1. User navigates to GarageDetail.svelte
2. User clicks "Register Bikes/Parts"
3. RegisterPartModal opens, fetches user's parts from `$parts` store
4. Modal shows two tabs: "Bikes" and "Spare Parts"
5. User switches between tabs to select bikes and/or spare parts
6. User checks boxes for parts to register (can select multiple)
7. User clicks "Register Selected"
8. For each selected part, call `garage.registerPart(part.id)`
9. Dispatch `window.dispatchEvent(new CustomEvent('garage-updated', { detail: { garageId: garage.id } }))`
10. GarageDetail listens for event and refreshes parts list
11. GarageCard also listens and updates part count

**From Part View** (works for both bikes and spare parts):
1. User viewing Part.svelte for their part (bike or spare part)
2. User clicks "Register to Garage" in menu
3. RegisterToGarageModal opens, fetches user's subscriptions
4. User selects a garage
5. Call `garage.registerPart(part.id)`
6. Dispatch event as above
7. Show success message with link to garage

### Unregistering a Part

**From Garage View** (garage owners OR part owners):
1. User viewing GarageDetail.svelte
2. User clicks menu (⋮) on a part card (bike or spare part)
3. User clicks "Unregister" (available if user is garage owner OR owns the part)
4. Show inline confirmation: "Remove [Part Name] from this garage?"
5. Call `garage.unregisterPart(part.id)`
6. Dispatch event
7. List refreshes

**Permission check**: "Unregister" option shown if:
- Current user is the garage owner, OR
- Current user owns the part being displayed

### Viewing Registered Parts

1. User navigates to GarageDetail.svelte
2. Component calls `garage.getParts()` → returns `number[]` of part IDs
3. For each ID, lookup full Part from `$parts` store
4. If Part not in store, fetch via Part API (may need new method)
5. **Privacy filter**: If user is NOT garage owner, filter to show only own parts:
   ```typescript
   const visibleParts = isOwner ? allParts : allParts.filter(p => p.owner === currentUserId);
   ```
6. Separate visible parts into two categories:
   - Bikes: where `part.isGear()` returns true
   - Spare Parts: where `part.isGear()` returns false
7. For each Part, lookup owner info if needed (only visible to garage owners)
8. Display in appropriate tab in RegisteredPartCard components

## Technical Implementation Details

### New Files to Create

1. **`frontend/src/Garage/GarageDetail.svelte`**
   - Main garage detail view
   - Props: `garageId: number`
   - State: `bikes: Part[]`, `spareParts: Part[]`, `loading: boolean`, `isOwner: boolean`, `hasActiveSubscription: boolean`, `activeTab: 'bikes' | 'spare-parts'`, `currentUserId: number`
   - Methods: `loadParts()`, `filterOwnParts()`, `handleUnregister(partId)`, `filterParts()`
   - Event listeners: 'garage-updated'
   - **Privacy filter**: Non-owners only see their own parts (filter by `part.owner === currentUserId`)

2. **`frontend/src/Garage/RegisterPartModal.svelte`** (renamed from RegisterBikeModal)
   - Modal for part registration (bikes and spare parts)
   - Props: `garage: Garage`, `isOpen: boolean`
   - State: `availableBikes: Part[]`, `availableSpareParts: Part[]`, `selectedParts: Set<number>`, `activeTab: 'bikes' | 'spare-parts'`
   - Methods: `loadAvailableParts()`, `handleRegister()`, `togglePartSelection(partId)`
   - Dispatches: 'close', 'registered'
   - Supports multi-select via checkboxes

3. **`frontend/src/Garage/RegisteredPartCard.svelte`** (renamed from RegisteredBikeCard)
   - Individual part card in garage view
   - Props: `part: Part`, `canUnregister: boolean`, `currentUserId: number`
   - Children: Dropdown menu snippet
   - Shows: part name, model, type badge, owner name, usage
   - Handles both bikes and spare parts with appropriate styling
   - `canUnregister` logic: `isGarageOwner || (part.owner === currentUserId)`

4. **`frontend/src/Garage/RegisterToGarageModal.svelte`**
   - Modal for registering from part view (bikes OR spare parts)
   - Props: `part: Part`, `isOpen: boolean`
   - State: `availableGarages: Garage[]`, `selectedGarage: Garage | null`
   - Methods: `loadAvailableGarages()`, `handleRegister()`
   - Works for both bikes and spare parts

### Files to Update

1. **`frontend/src/Garage/GarageCard.svelte`**
   - Add "View Details" to dropdown menu
   - Add navigation handler: `push(\`/garage/\${garage.id}\`)`

2. **`frontend/src/Part/Part.svelte`**
   - Add "Register to Garage" menu item
   - Add state for RegisterToGarageModal: `showGarageModal = $state(false)`

3. **`frontend/src/App.svelte`** (or router file)
   - Add route: `/garage/:id` → GarageDetail component

4. **`frontend/src/lib/garage.ts`**
   - Add helper method: `static async getDetail(id: number): Promise<Garage>`
   - Possibly add: `async getPartsWithDetails(): Promise<Part[]>`

5. **`frontend/src/lib/part.ts`**
   - Possibly add: `static async getByIds(ids: number[]): Promise<Part[]>`

### API Considerations

**Current limitation**: `GET /api/garage/{id}/parts` returns only IDs (`Vec<PartId>`), not full Part objects.

**Options**:

**Option A: Fetch parts separately** (Recommended for MVP)
- Frontend receives part IDs
- Lookup each part in `$parts` store
- If not in store, make individual API calls to `/api/part/{id}`
- Pro: No backend changes needed
- Con: Multiple API calls if parts not in cache

**Option B: Add new endpoint**
- Create `GET /api/garage/{id}/parts/details` returning `Vec<Part>`
- Backend joins garage_parts with parts table
- Returns full Part objects with owner info
- Pro: Single API call, better performance
- Con: Requires backend changes

**Recommendation**: Start with Option A for MVP, optimize to Option B if performance is an issue.

## Permission Matrix

| Action | Garage Owner | Active Subscriber (Part Owner) | Active Subscriber (Not Part Owner) | Pending/Rejected/Cancelled | Not Subscribed |
|--------|-------------|-------------------------------|-----------------------------------|---------------------------|----------------|
| View garage details | ✓ | ✓ | ✓ | ✗ | ✗ |
| View ALL registered parts | ✓ | ✗ | ✗ | ✗ | ✗ |
| View OWN registered parts | ✓ | ✓ | N/A | ✗ | ✗ |
| View OTHER USERS' parts | ✓ | ✗ | ✗ | ✗ | ✗ |
| Register own parts | ✓ | ✓ | N/A | ✗ | ✗ |
| Unregister any part | ✓ | ✗ | ✗ | ✗ | ✗ |
| Unregister own part | ✓ | ✓ | N/A | ✗ | ✗ |
| View part owner info | ✓ | Own parts only | ✗ | ✗ | ✗ |
| Register part to multiple garages | ✓ | ✓ | N/A | ✗ | ✗ |

## Event System

Following existing pattern in garage subscriptions:

**Custom Event**: `'garage-updated'`
- **Payload**: `{ garageId: number }`
- **Dispatched when**:
  - Part (bike or spare part) registered to garage
  - Part unregistered from garage
- **Listened by**:
  - GarageCard (updates part count)
  - GarageDetail (refreshes parts list)

**Example**:
```typescript
// Dispatch
window.dispatchEvent(new CustomEvent('garage-updated', {
  detail: { garageId: garage.id }
}));

// Listen
onMount(() => {
  window.addEventListener('garage-updated', handleGarageUpdate as EventListener);
});

function handleGarageUpdate(event: CustomEvent) {
  if (event.detail.garageId === garage.id) {
    loadParts();  // Reloads both bikes and spare parts
  }
}
```

## User Stories & Workflows

### Story 1: Garage Owner Views Registered Parts

**As a garage owner, I want to see all bikes and spare parts registered to my garage**

1. User navigates to "My Garages" tab
2. User clicks "View Details" on one of their garages
3. GarageDetail page loads showing:
   - Garage name and description
   - Tabs for "Bikes" and "Spare Parts"
   - **Garage owners see**: ALL registered parts with owner names
   - For each part: name, model, type badge, owner name, usage
4. User can switch between Bikes and Spare Parts tabs
5. User can click on any part to view full details
6. User can unregister any part using the menu

### Story 2: Subscriber Registers Parts to Garage

**As a garage subscriber, I want to register my bikes and spare parts for maintenance**

**Path A: From Garage View**
1. User navigates to "My Subscriptions" tab
2. User finds their active subscription
3. User clicks garage to view details
4. User clicks "Register Bikes/Parts" button
5. Modal opens with tabs for "Bikes" and "Spare Parts"
6. User selects "Bikes" tab, checks 2 bikes
7. User switches to "Spare Parts" tab, checks 3 spare parts
8. User clicks "Register Selected" (5 items)
9. All selected parts appear in the registered parts list
10. Success message shown: "Successfully registered 5 items"

**Path B: From Part View** (works for both bikes and spare parts)
1. User navigates to their part (bike or spare part)
2. User opens part menu (⋮)
3. User clicks "Register to Garage"
4. Modal opens showing garages user can access
5. User selects garage and confirms
6. Success message shown with link to garage
7. Works identically for bikes and spare parts

### Story 3: Subscriber Views and Removes Their Own Parts

**As a subscriber, I want to see and remove only my bikes and spare parts from a garage**

1. User viewing GarageDetail for a garage they subscribe to (but don't own)
2. User switches to "Spare Parts" tab
3. **Privacy filter**: User ONLY sees their own spare parts, not other customers' parts
4. User clicks menu (⋮) on their spare part card (e.g., "My Worn Chain")
5. User sees "Unregister" option (available because they own this part)
6. User clicks "Unregister"
7. Inline confirmation appears: "Remove My Worn Chain from this garage?"
8. User confirms
9. Spare part removed from their view
10. Success message shown
11. Same workflow works for bikes in "Bikes" tab

**Note**: Subscriber does NOT see other users' parts at all - they only see their own parts

### Story 3b: Garage Owner Removes Any Parts

**As a garage owner, I want to remove any bikes and spare parts from my garage**

1. Garage owner viewing GarageDetail for their garage
2. Sees "Unregister" option on ALL parts (both their own and others')
3. Can remove any part regardless of ownership
4. Same confirmation workflow as above

### Story 4: User Checks if Part Can Be Registered

**As a user, I want to know which garages I can register my part to**

1. User viewing their spare part (e.g., a chain) in Part.svelte
2. User clicks "Register to Garage" (available for spare parts too!)
3. Modal shows:
   - Garages where user is owner
   - Garages where user has active subscription
   - Grayed out: garages where subscription is pending/rejected
4. If no garages available, show message: "You need an active garage subscription to register this part"
5. Works identically for bikes and spare parts

### Story 5: Workshop Manages Multiple Customers' Parts

**As a bike shop owner, I want to track all customer bikes AND their spare parts inventory**

1. Shop owner has a garage "City Bike Shop"
2. Multiple customers subscribe:
   - Customer A registers 1 road bike + 2 chains
   - Customer B registers 1 MTB + 4 tires
   - Customer C registers 1 gravel bike + 1 cassette
3. Shop owner views garage, switches to "Spare Parts" tab
4. **Sees ALL spare parts from ALL customers** with owner names:
   - "Chain (Customer A)"
   - "Chain (Customer A)"
   - "Tire (Customer B)"
   - "Tire (Customer B)"
   - "Tire (Customer B)"
   - "Tire (Customer B)"
   - "Cassette (Customer C)"
5. Can track which parts are available vs. need ordering
6. Can see who owns what for maintenance scheduling

**Privacy note**: Only garage owners see all parts. Customers only see their own parts when they view the garage.

## Error Handling

### Frontend Errors

1. **No parts to register**:
   - Message: "You don't have any bikes or spare parts available to register"
   - Action: Link to create new part

2. **No bikes in bike tab**:
   - Message: "You don't have any bikes to register. Check the Spare Parts tab or create a new bike."
   - Action: Link to Spare Parts tab

3. **No spare parts in spare parts tab**:
   - Message: "You don't have any spare parts to register. Check the Bikes tab or create a new spare part."
   - Action: Link to Bikes tab

4. **No garages available**:
   - Message: "You need to subscribe to a garage first"
   - Action: Link to find garages

5. **Part already registered**:
   - Message: "This [bike/spare part] is already registered to [Garage Name]"
   - Action: Show which garage, option to view

### Backend Errors

From [backend/domain/src/entities/garage.rs](backend/domain/src/entities/garage.rs:174-176):

1. **No subscription** (403 Forbidden):
   - "You must subscribe to this garage before registering bikes"
   - Frontend: Show subscription request button

2. **Not part owner** (403 Forbidden):
   - "You can only register your own bikes and spare parts"
   - Frontend: Filter parts by current user

3. **Garage not found** (404 Not Found):
   - "Garage does not exist"
   - Frontend: Navigate back to garage list

## Styling & UI Components

### Using Flowbite Components

Following existing patterns:

- **Card**: For part cards in list
- **Badge**: For part type (Road Bike, Chain, Wheel, etc.), status
- **Button**: Primary action buttons
- **Dropdown**: Action menus
- **Modal**: For registration dialogs with tabs
- **Tabs**: For separating bikes and spare parts
- **Checkbox**: For multi-select in registration modal
- **Table**: Alternative layout for parts list (if many parts)

### Responsive Design

- **Desktop**: Grid layout for part cards (2-3 columns)
- **Tablet**: 2 columns
- **Mobile**: Single column, stacked

### Color Coding

- **Owner badge**: Green (existing pattern)
- **Subscriber badge**: Blue
- **Part count badge**: Blue (existing)
- **Bike type badges**: Green (Road Bike, MTB, etc.)
- **Spare part type badges**: Gray or Purple (Chain, Wheel, Tire, etc.)
- **Usage warnings**: Yellow/Red if service overdue (future enhancement)

## Testing Checklist

### Part Registration (Bikes and Spare Parts)

- [ ] Garage owner can register their own bikes
- [ ] Garage owner can register their own spare parts
- [ ] Active subscriber can register their own bikes
- [ ] Active subscriber can register their own spare parts
- [ ] Cannot register without active subscription
- [ ] Cannot register someone else's parts
- [ ] Cannot register disposed parts
- [ ] Cannot register part twice to same garage
- [ ] Can register multiple parts at once (batch)
- [ ] Success message shows count of registered items
- [ ] Part count updates in GarageCard
- [ ] Parts appear in correct tab (Bikes or Spare Parts)

### Part Unregistration

- [ ] Garage owner can unregister any bike (their own and others')
- [ ] Garage owner can unregister any spare part (their own and others')
- [ ] Part owners can unregister their own bikes
- [ ] Part owners can unregister their own spare parts
- [ ] Subscribers cannot unregister other users' parts
- [ ] "Unregister" option only shown when user has permission
- [ ] Confirmation shown before unregister
- [ ] Part removed from list after unregister
- [ ] Part count updates in GarageCard
- [ ] Can unregister and re-register same part

### Views and Navigation

- [ ] GarageDetail shows ALL parts for garage owner
- [ ] GarageDetail shows ONLY own parts for subscriber
- [ ] Subscribers cannot see other users' parts
- [ ] Bikes tab shows only bikes (isGear() === true)
- [ ] Spare Parts tab shows only spare parts (isGear() === false)
- [ ] Tab switching works correctly
- [ ] Non-subscribers cannot access GarageDetail
- [ ] Navigate from GarageCard to GarageDetail
- [ ] Navigate from part card to Part detail
- [ ] Back button works correctly

### Permissions

- [ ] Garage owner sees ALL parts with ALL owner names
- [ ] Garage owner sees "Unregister" option for all parts
- [ ] Subscriber sees ONLY their own parts (privacy filter works)
- [ ] Subscriber does NOT see other users' parts at all
- [ ] Part owner sees "Unregister" option for their own parts
- [ ] "Register Bikes/Parts" button shown appropriately
- [ ] Registration available for both bikes and spare parts
- [ ] Parts can be registered to multiple garages (no constraint)

### UI/UX

- [ ] Modal opens and closes correctly
- [ ] Tabs switch correctly in modal (Bikes/Spare Parts)
- [ ] Checkboxes work for multi-select
- [ ] Loading states shown during API calls
- [ ] Error messages displayed appropriately
- [ ] Success messages show count of registered items
- [ ] Real-time updates via events
- [ ] Responsive layout works on mobile
- [ ] Empty states handled (no bikes/spare parts registered)
- [ ] Part type badges display correctly

## Implementation Phases

### Phase 1: Basic Viewing (MVP)

**Goal**: Users can view bikes and spare parts registered to garages

**Tasks**:
1. Create GarageDetail.svelte with tabs
2. Create RegisteredPartCard.svelte (handles both bikes and spare parts)
3. Add route in App.svelte
4. Add "View Details" to GarageCard
5. Implement part loading logic with `isGear()` filtering
6. **Implement privacy filter**: Non-owners only see own parts
7. Implement tab switching (Bikes/Spare Parts)
8. Add part type badges
9. Test viewing as owner (sees all) and subscriber (sees only own)

**Estimate**: 6-8 hours (increased due to tabs, filtering, and privacy logic)

### Phase 2: Registration from Garage

**Goal**: Subscribers can register bikes and spare parts from garage view

**Tasks**:
1. Create RegisterPartModal.svelte with tabs
2. Add "Register Bikes/Parts" button to GarageDetail
3. Implement part filtering with `isGear()` separation
4. Implement multi-select with checkboxes
5. Implement batch registration logic
6. Add event dispatching
7. Test registration workflow for both bikes and spare parts

**Estimate**: 4-5 hours (increased due to tabs, filtering, and multi-select)

### Phase 3: Unregistration

**Goal**: Garage owners and part owners can remove parts

**Tasks**:
1. Add unregister option to part menu (conditionally shown)
2. Implement permission check: show if `isGarageOwner || part.owner === currentUserId`
3. Update backend to allow part owners to unregister their own parts
4. Implement inline confirmation
5. Implement unregister logic
6. Update event handling
7. Test unregistration workflow for both garage owners and part owners

**Estimate**: 3-4 hours (increased due to permission logic and backend update)

### Phase 4: Registration from Part View

**Goal**: Users can register bikes AND spare parts from Part.svelte

**Tasks**:
1. Create RegisterToGarageModal.svelte
2. Add menu item to Part.svelte (for both bikes and spare parts)
3. Implement garage list loading
4. Implement registration from part view
5. Test workflow for both bikes and spare parts

**Estimate**: 2-3 hours

### Phase 5: Polish & Edge Cases

**Goal**: Handle all edge cases and improve UX

**Tasks**:
1. Add loading states
2. Improve error messages
3. Add empty states
4. Test all permission scenarios
5. Mobile responsiveness
6. Add owner info display
7. Documentation updates

**Estimate**: 2-3 hours

**Total Estimated Time**: 17-23 hours (increased due to spare parts support, part owner unregister permissions, and privacy filtering)

## Future Enhancements

### Short-term
1. **Batch operations**: Already planned - register multiple parts at once ✓
2. **Part search/filter**: In GarageDetail view (search across bikes and spare parts)
3. **Sort options**: By name, owner, usage, type, etc.
4. **Export parts list**: CSV or PDF
5. **Part notes**: Garage owners add notes to registered parts
6. **Spare part inventory**: Track quantity of spare parts (e.g., "3 spare chains")

### Medium-term
1. **Service scheduling**: Schedule maintenance for garage bikes
2. **Usage tracking**: Automatic updates from Strava
3. **Maintenance alerts**: Notify owners when service due
4. **Bike transfer**: Transfer bike from one garage to another
5. **History log**: Track registration/unregistration events

### Long-term
1. **Fleet management**: Advanced tools for garages with many bikes
2. **Service packages**: Garage owners offer service packages
3. **Billing integration**: Charge subscribers for services
4. **Inventory management**: Track parts used on bikes
5. **Analytics dashboard**: Usage stats, service history

## Backend Changes Required

### Update `unregister_part` Permission Logic

Currently in [backend/domain/src/entities/garage.rs](backend/domain/src/entities/garage.rs:182-191), the `unregister_part` method only allows garage owners:

```rust
pub async fn unregister_part(
    self,
    part_id: PartId,
    user: &dyn Person,
    store: &mut impl Store,
) -> TbResult<()> {
    self.checkuser(user, store).await?;  // Only checks garage ownership
    store.garage_unregister_part(self, part_id).await
}
```

**Required change**: Allow EITHER garage owner OR part owner to unregister:

```rust
pub async fn unregister_part(
    self,
    part_id: PartId,
    user: &dyn Person,
    store: &mut impl Store,
) -> TbResult<()> {
    // Check if user is garage owner OR part owner
    let garage = store.garage_get(self).await?;
    let is_garage_owner = garage.owner == user.get_id() || user.is_admin();

    // Check if user owns the part
    let part = part_id.read(user, store).await.ok();
    let is_part_owner = part.map(|p| p.owner == user.get_id()).unwrap_or(false);

    if !is_garage_owner && !is_part_owner {
        return Err(Error::Forbidden(
            "You must be the garage owner or part owner to unregister this part".into(),
        ));
    }

    store.garage_unregister_part(self, part_id).await
}
```

## Resolved Questions

1. **Can a part be registered to multiple garages?** ✓ **DECIDED: YES**
   - Parts CAN be registered to multiple garages
   - No constraint will be added
   - Use case: Bike registered to personal garage AND shop
   - Use case: Spare parts inventory shared between garages

2. **Should subscribers see other parts in garage?** ✓ **DECIDED: NO**
   - **Subscribers only see their OWN parts** (privacy-first approach)
   - **Garage owners see ALL parts with owner names**
   - This provides privacy for customers while giving workshops full visibility
   - Customers cannot see other customers' bikes/parts
   - Privacy filter implemented in frontend: `parts.filter(p => p.owner === currentUserId)`

3. **Spare parts vs bikes UX?** ✓ **DECIDED: TABS**
   - Use tabbed interface with "Bikes" and "Spare Parts" tabs
   - Better separation and clearer organization
   - Consistent with registration modal design

## Open Questions

1. **Backend API optimization**:
   - Should we create `/api/garage/{id}/parts/details` endpoint?
   - When should we optimize vs. MVP approach?

2. **User notifications**:
   - Should garage owner be notified when parts are registered?
   - Should user be notified when their parts are unregistered?
   - Email or in-app notifications?
   - Batch notifications if multiple parts registered at once?

3. **Part ownership transfer**:
   - If part ownership changes, what happens to garage registration?
   - Should registration be automatically removed?
   - More relevant for spare parts that might be sold/transferred

## Summary

This plan provides a complete roadmap for implementing bike and spare parts registration UI for the garage system. The backend API is already in place and works for both bikes and spare parts, so the focus is entirely on creating intuitive frontend components that follow existing patterns in the codebase.

**Key Implementation Points**:
- Support BOTH bikes (gear) and spare parts using `part.isGear()` method
- **Use tabbed interface** to separate bikes and spare parts (confirmed approach)
- Enable multi-select registration for efficiency
- **Privacy-first**: Subscribers only see their own parts, not other users' parts
- **Allow part owners to unregister their own parts** (not just garage owners)
- **Allow parts to be registered to multiple garages** (no constraint)
- **Update backend permission logic** to support part owner unregistration
- Leverage existing Part model and garage infrastructure
- Follow Svelte 5 patterns with runes
- Use custom window events for real-time updates
- Implement inline confirmations for better UX
- Start with MVP approach, optimize later
- Maintain clear permission boundaries with proper ownership checks and privacy filtering

**Next Steps**:
1. ✅ Plan approved with key decisions made
2. ✅ Open questions resolved (multi-garage: YES, privacy: subscribers see only own parts, UI: tabs)
3. Begin Phase 1 implementation (Basic Viewing with privacy filter)
4. Iterate based on user feedback
