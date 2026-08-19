# Detailed Conversion Plan: Complex Files

## File 1: [`frontend/src/Widgets/Actions.svelte`](frontend/src/Widgets/Actions.svelte:1)

### Current Architecture

```
<module script>                      <instance script>
+----------------------------------+  +------------------------------------------+
| export let actions =            |  | let newPart: { start: ... }              |
|   writable<ModalType>()         |  | let serviceActions: {...}                |
|                                  |  | ...17 child refs total                   |
| 15 child components              |  | bind:this=newPart                        |
| imported by other files          |  | $: actions.set({ref mappings...})        |
+----------------------------------+  +------------------------------------------+

Consumers: 15 files import { actions } from Actions.svelte
Usage: 29 locations use $actions.xxx pattern
```

### Conversion Strategy: Option 1 - One $state per Method

**Convert the single `writable<ModalType>` store into 21 individual `$state` exports.**

#### Module Script Changes

**Before:**
```typescript
<script context="module" lang="ts">
  import { writable } from "svelte/store";

  type ModalType = { ... };  // 21 methods

  export let actions = writable<ModalType>();
</script>
```

**After:**
```typescript
<script context="module" lang="ts">
  import type { Part } from "../lib/part";
  import type { Attachment } from "../lib/attachment";
  import type { Activity } from "../lib/activity";
  import type { Service } from "../lib/service";
  import type { ServicePlan } from "../lib/serviceplan";
  import type { Type } from "../lib/types";
  import type { Shop } from "../lib/shop";

  // 21 individual $state exports replacing the writable store
  export let newPart = $state<((t: Type) => void) | undefined>(undefined);
  export let installPart = $state<((p: Part) => void) | undefined>(undefined);
  export let changePart = $state<((p: Part) => void) | undefined>(undefined);
  export let deletePart = $state<((p: Part) => void) | undefined>(undefined);
  export let disposePart = $state<((p: Part, a?: Attachment) => void) | undefined>(undefined);
  export let recoverPart = $state<((p: Part) => void) | undefined>(undefined);
  export let replacePart = $state<((p: Attachment) => void) | undefined>(undefined);
  export let attachPart = $state<((p: Part) => void) | undefined>(undefined);
  export let newService = $state<((part: Part, plan?: ServicePlan) => void) | undefined>(undefined);
  export let newPlan = $state<((p: Part) => void) | undefined>(undefined);
  export let changeService = $state<((s: Service) => void) | undefined>(undefined);
  export let redoService = $state<((s: Service) => void) | undefined>(undefined);
  export let deleteService = $state<((s: Service) => void) | undefined>(undefined);
  export let updatePlan = $state<((p: ServicePlan) => void) | undefined>(undefined);
  export let deletePlan = $state<((p: ServicePlan) => void) | undefined>(undefined);
  export let deleteAttachment = $state<((a: Attachment) => void) | undefined>(undefined);
  export let changeActivity = $state<((a: Activity) => void) | undefined>(undefined);
  export let createShop = $state<(() => void) | undefined>(undefined);
  export let editShop = $state<((g: Shop) => void) | undefined>(undefined);
  export let deleteShop = $state<((g: Shop) => void) | undefined>(undefined);
  export let requestSubscription = $state<((g: Shop) => void) | undefined>(undefined);
</script>
```

#### Instance Script Changes

**Before:**
```typescript
$: actions.set({
  newPart: newPart?.start,
  installPart: installPart?.start,
  // ... 19 more mappings
});
```

**After:**
```typescript
$effect(() => { newPart = newPart?.start; });
$effect(() => { installPart = installPart?.start; });
$effect(() => { changePart = changePart?.start; });
$effect(() => { deletePart = deletePart?.start; });
$effect(() => { disposePart = disposePart?.start; });
$effect(() => { recoverPart = recoverPart?.start; });
$effect(() => { attachPart = attachPart?.start; });
$effect(() => { replacePart = replacePart?.start; });
$effect(() => { newService = serviceActions?.create; });
$effect(() => { redoService = serviceActions?.repeat; });
$effect(() => { changeService = serviceActions?.change; });
$effect(() => { deleteService = serviceActions?.del; });
$effect(() => { newPlan = newPlan?.start; });
$effect(() => { updatePlan = updatePlan?.start; });
$effect(() => { deletePlan = deletePlan?.start; });
$effect(() => { deleteAttachment = deleteAttachment?.start; });
$effect(() => { changeActivity = changeActivity?.start; });
$effect(() => { createShop = () => shopModal?.start(); });
$effect(() => { editShop = shopModal?.start; });
$effect(() => { deleteShop = deleteShop?.start; });
$effect(() => { requestSubscription = subscriptionRequestModal?.start; });
```

#### Consumer File Updates Required

Each importing file must change from:
```typescript
import { actions } from "../Widgets/Actions.svelte";
// then use $actions.newPart(...) etc.
```

To individual imports:
```typescript
import { newPart, deletePart } from "../Widgets/Actions.svelte";
// then use $newPart(...) instead of $actions.newPart(...)
```

### Files Requiring Import Updates (14 files)

| # | File | Actions Used | New Imports |
|---|------|-------------|-------------|
| 1 | [`Part/Part.svelte`](frontend/src/Part/Part.svelte:12) | newPart, deletePart, disposePart, changePart, installPart, newPlan, newService | 7 imports |
| 2 | [`Part/PartCard.svelte`](frontend/src/Part/PartCard.svelte:13) | newService, attachPart, replacePart | 3 imports |
| 3 | [`Part/PartHist.svelte`](frontend/src/Part/PartHist.svelte:10) | deleteAttachment | 1 import |
| 4 | [`Activity/ActName.svelte`](frontend/src/Activity/ActName.svelte:5) | changeActivity | 1 import |
| 5 | [`Activity/ActCard.svelte`](frontend/src/Activity/ActCard.svelte:28) | changeActivity | 1 import |
| 6 | [`Service/ServiceMenu.svelte`](frontend/src/Service/ServiceMenu.svelte:3) | changeService, redoService, deleteService | 3 imports |
| 7 | [`ServicePlan/PlanRow.svelte`](frontend/src/ServicePlan/PlanRow.svelte:12) | newService, replacePart, installPart | 3 imports |
| 8 | [`ServicePlan/PlanBlock.svelte`](frontend/src/ServicePlan/PlanBlock.svelte:9) | updatePlan, deletePlan | 2 imports |
| 9 | [`Shop/ShopOwnerMenu.svelte`](frontend/src/Shop/ShopOwnerMenu.svelte:11) | editShop, deleteShop | 2 imports |
| 10 | [`Shop/Subscriptions.svelte`](frontend/src/Shop/Subscriptions.svelte:19) | requestSubscription | 1 import |
| 11 | [`Shop/Shops.svelte`](frontend/src/Shop/Shops.svelte:8) | createShop | 1 import |
| 12 | [`Shop/ShopList.svelte`](frontend/src/Shop/ShopList.svelte:8) | requestSubscription | 1 import |
| 13 | [`Spares/SpareType.svelte`](frontend/src/Spares/SpareType.svelte:13) | newPart, attachPart, deletePart, disposePart | 4 imports |
| 14 | [`App.svelte`](frontend/src/App.svelte:64) | N/A | No change (component import, not store) |

**Total consumption sites to update**: 29 usages across 13 files

### Trade-offs of Option 1

| Aspect | Pros | Cons |
|--------|------|------|
| **Modularity** | Each action is independently subscribable | More imports per file |
| **Tree-shaking** | Unused actions can be dropped | More exports to maintain |
| **Readability** | Clear what each file uses | 13 files need import changes |
| **Type safety** | Each state has precise type | No single ModalType to reference |

---

## File 2: [`frontend/src/Activity/Activities.svelte`](frontend/src/Activity/Activities.svelte:1)

### Current Code

```svelte
<script lang="ts">
  export let params: { part: number; start?: number };

  let acts: Activity[];
  let title: string;
  $: if (params.part) {
    let part = $parts[params.part];
    title = m.act_heading_for({ name: part.name });
    if (part.isGear()) {
      acts = filterValues($activities, (a) => a.gear == part.id);
    } else {
      let start = Number(params.start);
      let atts = part.attachments($attachments).filter(...);
      acts = atts.map((att) => att.activities($activities)).flat();
      if (start)
        title = m.act_heading_attached({ name: part.name, part: ..., date: ... });
    }
  } else {
    title = m.act_heading_all();
    acts = $category.activities($activities);
  }
</script>

<ActList {acts} {title} />
```

### Conversion Challenge

The `$:` block is an **imperative reactive statement** that:
1. Reads `params.part`, `params.start`
2. Reads global stores `$parts`, `$activities`, `$category`, `$attachments`
3. Computes two output values: `acts` and `title`
4. Has nested conditionals with local variables

### Conversion Strategy: Helper Function + $derived

```svelte
<script lang="ts">
  import { category } from "../lib/types";
  import { Activity, activities } from "../lib/activity";
  import ActList from "./ActList.svelte";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import * as m from "../../paraglide/messages";

  let { params }: { params: { part: number; start?: number } } = $props();

  // Helper function for the reactive computation
  function computeActivitiesAndTitle() {
    let acts: Activity[];
    let title: string;

    if (params.part) {
      const part = $parts[params.part];
      title = m.act_heading_for({ name: part.name });
      if (part.isGear()) {
        acts = filterValues($activities, (a) => a.gear == part.id);
      } else {
        const start = Number(params.start);
        const atts = part
          .attachments($attachments)
          .filter((a) => (start ? a.isAttached(start) : true));
        acts = atts.map((att) => att.activities($activities)).flat();
        if (start)
          title = m.act_heading_attached({
            name: part.name,
            part: $parts[atts[0].gear]
              ? $parts[atts[0].gear].name
              : m.act_unknown_part(),
            date: atts[0].fmtTime(),
          });
      }
    } else {
      title = m.act_heading_all();
      acts = $category.activities($activities);
    }

    return { acts, title };
  }

  // Reactive derivation
  let { acts, title } = $derived(computeActivitiesAndTitle());
</script>

<ActList {acts} {title} />
```

### Why This Works

1. The helper function captures all reactive dependencies ($parts, $activities, etc.)
2. `$derived()` automatically re-runs when any accessed store changes
3. Keeps existing logic structure intact
4. Returns both computed values in a single reactive unit

### Files Affected

Only [`Activities.svelte`](frontend/src/Activity/Activities.svelte:40) itself - the child `<ActList {acts} {title} />` receives the same props as before. No consumer changes.

---

## Conversion Order

```
Phase 1: Actions.svelte module conversion
    |
    +-- Phase 2: Update all 13 consumer file imports
    |
    +-- Phase 3: Activities.svelte reactive block
    |
    +-- Phase 4: PartCard.svelte (4 dependent $derived values)
    |
    +-- Phase 5: Remaining simple files (export let -> $props)
    |
    +-- Phase 6: Lifecycle hooks (onMount/onDestroy -> $effect cleanup)
```

### Phase Breakdown

| Phase | Files | Complexity | Dependencies |
|-------|-------|------------|--------------|
| 1 | Actions.svelte | High | None |
| 2 | 13 consumer files | Medium | Phase 1 |
| 3 | Activities.svelte | Medium | None |
| 4 | PartCard.svelte | Medium | - |
| 5 | 13 simple files | Low | - |
| 6 | 4 lifecycle files | Low | - |

**Total unique files**: 17 files
**Total code changes**: ~200 lines across 17 files
