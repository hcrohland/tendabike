# User Onboarding Flow

## Overview

This document describes the user onboarding flow implemented in TendaBike to give users control over when their historic Strava activities are imported.

## Problem Statement

Previously, when a new user registered with TendaBike, the application would automatically pull all historic activities from Strava without any user interaction or explanation. This could be:
- Unexpected for users who just wanted to try the app
- Time-consuming for users with many activities
- Confusing without context about what was happening

## Solution

We implemented a user-controlled onboarding flow that:
1. Shows a welcome dialog to new users explaining what historic activity import does
2. Allows users to choose whether to import immediately or postpone
3. Provides ways to trigger the import later if postponed
4. Tracks the user's onboarding status in the database

## Architecture

### Onboarding Status Enum

The system uses a state machine with three states:

```rust
pub enum OnboardingStatus {
    Pending = 0,                    // User has not completed initial activity sync
    InitialSyncPostponed = 2,       // User chose to postpone initial activity sync
    Completed = 99,                 // User has completed onboarding
}
```

**Note:** The discriminant values are intentionally non-sequential:
- `0` = Default state for new users
- `2` = Postponed state
- `99` = Completed state (high value to allow for future intermediate states)

### Database Schema

The `users` table includes an `onboarding_status` column:

```sql
ALTER TABLE users
ADD COLUMN IF NOT EXISTS onboarding_status INTEGER NOT NULL DEFAULT 0;
```

This integer field maps to the `OnboardingStatus` enum discriminants.

## User Flow

### 1. New User Registration

When a user completes Strava OAuth:
- User record is created with `onboarding_status = Pending` (0)
- **No automatic activity sync is triggered**
- User is redirected to the application

### 2. Welcome Dialog

Upon first login, users with `Pending` status see a modal dialog that:
- Welcomes them to TendaBike
- Explains what historic activity import does
- Notes that the sync runs in the background
- Mentions it may take several minutes
- Explains that new activities will sync automatically regardless of choice
- Offers two options:
  - **Import Activities**: Triggers historic sync, sets status to `Completed` (99)
  - **Skip for Now**: Sets status to `InitialSyncPostponed` (2), can import later

### 3. Postponed Users

Users who chose "Skip for Now" can trigger the import later via:

**User Menu Dropdown:**
- Navigate to Sync → "Import Historic Activities"
- Available only when status is `InitialSyncPostponed`

**Navigation Bar Button:**
- Appears prominently in the navbar when:
  - Status is `Pending` OR `InitialSyncPostponed`
  - AND user has no activities
- Click triggers the historic sync
- Button disappears once activities exist or status becomes `Completed`

## API Endpoints

### POST /strava/onboarding/sync

Triggers the initial historic activity sync for the authenticated user.

**Query Parameters:**
- `time` (optional): Unix timestamp for how far back to sync (default: 0 = all time)

**Returns:** Updated `User` object with `onboarding_status = Completed`

**Errors:**
- `400 Bad Request` if sync already triggered (status is `Completed`)

### POST /strava/onboarding/postpone

Marks the initial sync as postponed.

**Returns:** Updated `User` object with `onboarding_status = InitialSyncPostponed`

**Errors:**
- `400 Bad Request` if status is not `Pending`

## Implementation Details

### Code Examples

**Dialog Component Structure:**
```svelte
<script lang="ts">
  import { Modal, P, Heading, Button } from "flowbite-svelte";
  import { myfetch, handleError, user } from "./store";

  export let open = false;
  let loading = false;

  async function triggerSync() {
    loading = true;
    try {
      const updatedUser = await myfetch("/strava/onboarding/sync", "POST");
      $user = updatedUser;
      open = false;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }

  async function skipSync() {
    loading = true;
    try {
      const updatedUser = await myfetch("/strava/onboarding/postpone", "POST");
      user.set(updatedUser);
      open = false;
    } catch (e) {
      handleError(e as Error);
    } finally {
      loading = false;
    }
  }
</script>

<Modal bind:open size="lg" autoclose={false} dismissable={false} outsideclose={false}>
  <!-- Dialog content with explanatory text -->
  <Button color="blue" disabled={loading} onclick={triggerSync}>
    {loading ? "Importing..." : "Import Activities"}
  </Button>
  <Button color="alternative" disabled={loading} onclick={skipSync}>
    Skip for Now
  </Button>
</Modal>
```

### Backend

**Domain Layer** (`backend/domain/src/entities/user.rs`):
- `OnboardingStatus` enum with `#[repr(i32)]` for database mapping
- `TryFrom<i32>` implementation for safe deserialization
- `From<OnboardingStatus> for i32` for serialization
- Helper method `is_initial_sync_completed()` for status checks

**Database Layer** (`backend/sqlx/src/store/user.rs`):
- `DbUser` struct includes `onboarding_status: i32` field
- Conversion implementations between `User` and `DbUser`
- `update_onboarding_status()` method to update user status

**API Layer** (`backend/axum/src/strava/webhook.rs`):
- `trigger_initial_sync()` endpoint handler
- `postpone_initial_sync()` endpoint handler
- Both return updated User object to avoid frontend reloads

### Frontend

**Type Definitions** (`frontend/src/lib/types.ts`):
```typescript
export type OnboardingStatus = "pending" | "completed" | "initial_sync_postponed";

export type User = {
  id: number;
  firstname: string;
  name: string;
  is_admin: boolean;
  avatar: string | undefined;
  onboarding_status: OnboardingStatus;
}
```

**Welcome Dialog** (`frontend/src/lib/InitialSyncDialog.svelte`):
- Flowbite Modal component with customized settings:
  - Non-dismissable (user must make a choice)
  - Cannot be closed by clicking outside
  - Positioned at top-center
- Two Flowbite Button components:
  - **Import Activities** (blue color): Calls `/strava/onboarding/sync` endpoint
  - **Skip for Now** (alternative color): Calls `/strava/onboarding/postpone` endpoint
- Both buttons show loading state with text changes during API calls
- Updates `$user` store directly with returned User object
- Closes automatically on successful API response
- Uses two-way binding (`bind:open`) for parent component control

**Main App** (`frontend/src/App.svelte`):
- Shows dialog when `$user.onboarding_status === "pending"`
- Uses reactive statement to respond to user store changes

**Navigation Header** (`frontend/src/Header.svelte`):
- Native HTML button in navbar for users with `Pending` or `InitialSyncPostponed` status and no activities
- Menu item in Sync dropdown for `InitialSyncPostponed` users only
- Both call `triggerHistoricSync()` function
- Updates user store and refreshes data after import
- Button uses native HTML for consistent styling with navbar elements

## Design Decisions

### Why use Native Buttons in navbar?

**Navbar Button (Native HTML):**
- The navigation bar uses a native `<button>` element
- Maintains consistency with other navbar elements
- Provides fine-grained control over positioning and styling
- Integrates seamlessly with Flowbite's navbar components

## Future Enhancements

Potential improvements to the onboarding system:

1. **Partial Import**: Allow users to specify a date range

## Testing

To test the onboarding flow:

1. Create a new test user via Strava OAuth
2. Verify welcome dialog appears with both options
3. Test "Skip for Now" → verify status = `InitialSyncPostponed`
4. Verify import button appears in navbar (when no activities)
5. Verify menu item appears in Sync dropdown
6. Trigger import from either location
7. Verify status changes to `Completed` (99)
8. Verify UI updates without page reload
9. Verify button/menu item disappears after completion

## Migration Notes

For existing users before this feature:
- All existing users have `onboarding_status = 99` (Completed) by default

For database migrations:
- The migration adds the column with default value 99
