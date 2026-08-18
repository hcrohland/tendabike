# Plan: Activities Card Layout

## Overview
Replace the table-based layout in the Activities component with a mobile-friendly card layout by creating new components, while keeping the existing table component unchanged.

## Rules from frontend/AGENTS.md

All new components must follow these conventions:
- **Svelte 5 runes mode**: Use `$state`, `$derived`, `$props` (not legacy `export let` / `$:`)
- **TypeScript**: `<script lang="ts">` blocks
- **License header**: AGPLv3 header block (from App.svelte line 1-19) required at top of each new `.svelte` file
- **i18n**: Import paraglide messages via `import * as m from "../../paraglide/messages"` (matching existing Activity component pattern)
- **Store updates**: Use entity store methods (`parts.updateMap()`, etc.) - new components read from global stores via `$parts`, `$activities`

## Files to Create/Modify

| Action | File | Description |
|--------|------|-------------|
| Create | [`ActCard.svelte`](frontend/src/Activity/ActCard.svelte) | Full-width card for one activity |
| Create | [`TotalsCard.svelte`](frontend/src/Activity/TotalsCard.svelte) | Totals row + sort/filter controls |
| Create | [`ActList.svelte`](frontend/src/Activity/ActList.svelte) | Card list container with date slider |
| Modify | [`Activities.svelte`](frontend/src/Activity/Activities.svelte:5) | Import ActList instead of ActTable |
| Keep | [`ActTable.svelte`](frontend/src/Activity/ActTable.svelte:1) | Unchanged (backwards compat) |
| Keep | [`ActName.svelte`](frontend/src/Activity/ActName.svelte:1) | Unchanged |

## Layout Design

### Visual Layout - ActList Container

```
<div class="flex flex-col gap-4 max-w-4xl mx-auto">
  +-- TotalsCard (sticky at top, contains: date slider, filters, sort, totals)
  +-- ActCard (full width card 1 - sorted)
  +-- ActCard (full width card 2 - sorted)
  +-- ...
  +-- Alert (if no activities after filter)
</div>
```

### TotalsCard.svelte - Sticky Header with Controls

```
<div class="rounded-lg border border-border-subtle bg-surface-2 p-4 sticky top-16 z-10">
  |
  +-- Row 1: Title + Totals
  |   +-- "TOTALS" label (bold uppercase)
  |   +-- Totals Grid (responsive):
  |       +-- Count | Time | Distance | Climb | Descend | kJ
  |       +-- Values are summed from all filtered activities
  |
  +-- Row 2: Filters (collapsible on mobile)
  |   +-- Gear filter dropdown (select)
  |   +-- Sort: Date dropdown (date, name, time, distance, climb, descend, kJ)
  |   +-- Sort order toggle button (asc/desc arrow)
  |
  +-- Row 3: Date Range Slider
      +-- RangeSlider (same as ActTable)
</div>
```

### ActCard.svelte - Activity Card (Full Width)

```
<div class="rounded-lg border border-border-subtle bg-surface-1 p-4">
  |
  +-- Header Row (flex justify-between)
  |   +-- Left:
  |   |   +-- Date + Time (small text, text-text-1)
  |   |   +-- Activity name (link to Strava + strava_grey.png icon)
  |   +-- Right:
  |       +-- Menu dropdown (edit action)
  |
  +-- Metrics Grid (responsive, click headers in TotalsCard to sort)
  |   +-- Time | Distance | Climb | Descend | Energy
  |   +-- Each: label (uppercase small) + formatted value
  |
  +-- Footer (flex justify-between)
      +-- Gear name (linked to part)
      +-- Device name
</div>
```

### Responsive Breakpoints

| Element | Mobile (<640px) | Tablet (640px+) | Desktop (768px+) |
|---------|-----------------|-----------------|------------------|
| ActCard metrics | 2 cols | 3 cols | 5 cols |
| TotalsCard totals | 2 cols | 3 cols | 6 cols (all visible) |
| TotalsCard controls | Stacked | Inline row | Inline row |

## Component Specifications

### ActCard.svelte

**Props:**
- `activity: Activity`

**Content:**
- Header: date, time, name, Strava link, menu
- Metrics: Time | Distance | Climb | Descend | Energy
- Footer: Gear link, device name

**Data Mapping:**

| Field | Format |
|-------|--------|
| activity.start | toLocaleDateString() + toLocaleTimeString() |
| activity.name | Link to /strava/activities/{id} |
| activity.time | fmtSeconds() |
| activity.distance | fmtNumber() |
| activity.climb | fmtNumber() |
| activity.descend | fmtNumber() |
| activity.energy | fmtNumber() |
| activity.gear | gearName($parts) |
| activity.device_name | Plain text |

### TotalsCard.svelte

**Props:**
- `filtered: Activity[]` - Activities to sum (already filtered by date)
- `gearFilter: number | undefined` - Current gear filter
- `sortBy: string` - Current sort column
- `sortDir: number` - Current sort direction (-1 or 1)

**State:**
- `gearFilter: number | undefined`
- `sortBy: string` (default: "start")
- `sortDir: number` (default: -1)
- `showFilters: boolean` (mobile toggle)

**Computed:**
- `totals: Usage` - Sum of all filtered activities

```typescript
$: totals = filtered.reduce((sum, act) => {
  sum.add(act);
  return sum;
}, new Usage());
```

**Content:**

Row 1 - Totals Display:
| Metric | Value | Source |
|--------|-------|--------|
| Count | `totals.count` | Number of activities |
| Time | `fmtSeconds(totals.time)` | Sum of durations |
| Distance | `fmtNumber(totals.distance)` | Sum of distances |
| Climb | `fmtNumber(totals.climb)` | Sum of elevation gain |
| Descend | `fmtNumber(totals.descend)` | Sum of elevation loss |
| Energy | `fmtNumber(totals.energy)` | Sum of kJ |

Row 2 - Filters & Sort (inline on desktop, collapsible on mobile):
- **Gear filter**: Select dropdown (All + unique gear items)
- **Sort by**: Select dropdown (Date, Name, Time, Distance, Climb, Descend, Energy)
- **Sort order**: Toggle button showing arrow icon (ascending/descending)

Row 3 - Date Range:
- RangeSlider (same as ActTable, same behavior)

### ActList.svelte

**Props:**
- `acts: Activity[]` - Full list of activities from parent

**Computed Pipeline:**
```typescript
$: filtered = filterRows(acts, dateValues);  // Date range filter
$: byGear = filtered.filter(a => !gearFilter || a.gear === gearFilter);  // Gear filter
$: displayed = sortActivities(byGear, sortBy, sortDir);  // Sort
```

**Logic Reused from ActTable:**
- `MiniMax()` function - Calculate date min/max from activities
- `filterRows()` function - Filter by date range
- `createFilterOptions()` function - Generate gear filter options
- RangeSlider with date formatter

## CSS Classes

| Element | Classes |
|---------|---------|
| Card wrapper | `rounded-lg border border-border-subtle p-4` |
| TotalsCard bg | `bg-surface-2` (darker to distinguish from activity cards) |
| ActivityCard bg | `bg-surface-1` (lighter) |
| Metrics grid | `grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-2` |
| Metric box | `text-center p-2 rounded bg-surface-2` (card) / `bg-surface-1` (totals) |
| Metric label | `text-xs uppercase text-text-1` |
| Metric value | `text-base font-semibold` |
| Totals section | `border-t border-border-subtle pt-2 mt-2` |

## Activities.svelte Change

Only two lines change:

```svelte
<!-- Line 5: Change import -->
- import ActTable from "./ActTable.svelte";
+ import ActList from "./ActList.svelte";

<!-- Line 46: Change component -->
- <ActTable {acts} />
+ <ActList {acts} />
```

## Svelte 5 Runes Usage

All components use Svelte 5 runes (not legacy `$:` reactive statements):

| Rune | Purpose | Example |
|------|---------|---------|
| `$state()` | Reactive mutable state | `let sortBy = $state("start");` |
| `$derived()` | Derived/computed values | `let totals = $derived(calculate());` |
| `$props()` | Component props | `let { activity }: Props = $props();` |

### Runes in ActCard.svelte
```typescript
let { activity }: { activity: Activity } = $props();
```

### Runes in TotalsCard.svelte
```typescript
let { filtered, sortBy, sortDir }: Props = $props();

let gearFilter = $state<number | undefined>(undefined);
let totals = $derived(
  filtered.reduce((sum, act) => { sum.add(act); return sum; }, new Usage())
);
```

### Runes in ActList.svelte
```typescript
let { acts }: { acts: Activity[] } = $props();

let dateValues = $state([0, 0]);
let gearFilter = $state<number | undefined>(undefined);
let sortBy = $state("start");
let sortDir = $state(-1);

let filtered = $derived(filterRows(acts, dateValues));
let byGear = $derived(filtered.filter(a => !gearFilter || a.gear === gearFilter));
let displayed = $derived(sortActivities(byGear, sortBy, sortDir));
```

## Implementation Order

1. Create ActCard.svelte - Individual activity card
2. Create TotalsCard.svelte - Totals display + sort/filter controls
3. Create ActList.svelte - Container with date filter + reactive pipeline
4. Update Activities.svelte - Swap ActTable import for ActList
5. Test and verify responsive layout and filtering
