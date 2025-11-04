# Plan: Dynamic Attachment Loading with Spare Parts Support

## Goal
Garages should show:
- Registered bikes with their **current** attachments (fetched dynamically)
- Registered spare parts (unattached parts)

## Current Problem
- When registering a bike, all attached parts are permanently stored in `garage_parts` table
- This creates a static snapshot that doesn't reflect changes when parts are attached/detached later
- Attached parts don't appear in garage view because cascading registration isn't working correctly

## Solution Overview
1. **Remove cascading registration** - Don't store attached parts in database
2. **Dynamic attachment loading** - Fetch current attachments on-the-fly when loading garage details
3. **Support spare parts** - Allow registering unattached parts explicitly

---

## Phase 1: Core Dynamic Loading (Implement Now)

### 1. Modify `register_part()` Method
**File:** `backend/domain/src/entities/garage.rs` (lines 200-296)

**Changes:**
- Remove cascading registration loop (lines 237-285 with debug logging)
- Only register the specific part being registered (bike OR spare)
- Keep attachment fetch for response Summary (for UI display)

**Before:** Registers main part + all currently attached parts to database
**After:** Registers ONLY the specified part to database

### 2. Modify `get_details()` Method
**File:** `backend/domain/src/entities/garage.rs` (lines 360-435)

**Changes:**
Add dynamic attachment loading after line 384 (`let part_ids = store.garage_get_parts(self).await?;`)

**Implementation:**
```rust
// Fetch all registered part IDs
let part_ids = store.garage_get_parts(self).await?;
let now = time::OffsetDateTime::now_utc();

// Fetch full part details
let mut parts = Vec::new();
for part_id in &part_ids {
    if let Ok(part) = part_id.read(store).await {
        // Privacy filter: non-owners only see their own parts
        if is_owner || part.owner == user.get_id() {
            parts.push(part);
        }
    }
}

// Get attachments - DYNAMIC for bikes, none for spares
let mut attachments = Vec::new();
for part in &parts {
    // If this is a main part (bike), fetch its CURRENT attachments
    if part.what.is_main()? {
        let (part_attachments, _) = crate::Attachment::for_part_with_usage(part.id, store).await?;
        // Only include currently attached parts (now < detached)
        for att in part_attachments {
            if now < att.a.detached {
                attachments.push(att);
            }
        }
    }
    // Spare parts: no attachments to add
}
```

**What This Does:**
- Bikes: Shows with their current attachments (dynamic)
- Spare parts: Shows as-is (no attachments)
- Attachments always reflect current bike state, not registration state

### 3. Clean Up Database (Optional)
Remove previously cascaded parts that were stored:

```sql
DELETE FROM garage_parts gp
WHERE part_id IN (
    SELECT DISTINCT part_id FROM attachments
);
```

This removes any component parts (wheels, chains, etc.) that were registered via cascading.

---

## Phase 2: Auto-Registration (Deferred - Requires Garage Mode Context)

These features require backend garage mode awareness - table for future implementation:

### Auto-Register on Detach
**When:** User detaches a part from a bike while in garage mode
**Action:** Automatically register the detached part as a spare to the current garage
**Location:** `backend/axum/src/domain/attachment.rs` - detach endpoint

### Auto-Unregister on Attach
**When:** User attaches a spare part to a bike while in garage mode
**Action:** Automatically unregister the part from garage (it will show via dynamic attachment)
**Location:** `backend/axum/src/domain/attachment.rs` - attach endpoint

**Note:** These require passing garage context to backend, which needs architecture discussion.

---

## Benefits

1. **Dynamic State**: Attachments always reflect current bike configuration
2. **Flexibility**: Users can attach/detach parts without re-registering bikes
3. **Spare Parts**: Users can explicitly register loose parts to garage
4. **Simpler Logic**: No cascading, no sync issues
5. **Less Database Overhead**: Only main parts and spares stored, not every attachment

---

## Implementation Steps

1. ✅ Remove debug logging from `garage.rs`
2. ✅ Remove cascading registration in `register_part()`
3. ✅ Implement dynamic attachment loading in `get_details()`
4. ✅ Test bike registration - should show bike + current attachments
5. ✅ Test spare part registration - should show part only
6. ✅ Test attach/detach - bike attachments should update automatically
7. (Optional) Clean up existing database entries
8. (Future) Implement auto-registration for attach/detach operations

---

## Testing Checklist

- [ ] Register a bike to garage - bike appears
- [ ] Bike shows with its current attachments
- [ ] Attach a part to the bike - it appears in garage view
- [ ] Detach a part from the bike - it disappears from garage view
- [ ] Register a spare part - it appears separately
- [ ] Parts list shows: bikes + their current attachments + registered spares
- [ ] Privacy filter works (subscribers see only their own parts)

---

## Files to Modify

1. `backend/domain/src/entities/garage.rs`
   - `register_part()` method (lines 200-296)
   - `get_details()` method (lines 360-435)

2. Database cleanup (optional):
   - SQL query to remove cascaded parts
