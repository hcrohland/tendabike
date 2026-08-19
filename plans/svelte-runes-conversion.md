# Svelte 5 Runes Conversion Plan

## Summary

This plan identifies all Svelte files in the TendaBike frontend still using old Svelte 5 Component API syntax and provides a conversion strategy to Runes mode.

## Current State

- **Total Svelte files**: 70 files
- **Files already using Runes**: ~48 files (using `$props()`, `$state()`, `$derived()`, `$effect()`)
- **Files needing conversion**: 17 files

## Conversion Categories

### Category A: `export let` → `$props()` (10 files)

These files use `export let` for props. Conversion: Replace `export let` with destructured `$props()`.

| File | Props to Convert | Complexity |
|------|-----------------|------------|
| [`frontend/src/Admin/CreateSync.svelte`](frontend/src/Admin/CreateSync.svelte:13) | `refresh: () => void` | Low |
| [`frontend/src/Activity/Activities.svelte`](frontend/src/Activity/Activities.svelte:10) | `params: { part: number; start?: number }` | Low |
| [`frontend/src/Activity/ActName.svelte`](frontend/src/Activity/ActName.svelte:8) | `row: Activity` | Low |
| [`frontend/src/Activity/ChangeField.svelte`](frontend/src/Activity/ChangeField.svelte:4-5) | `field: number | undefined`, `label: string` | Low |
| [`frontend/src/Admin/Sync.svelte`](frontend/src/Admin/Sync.svelte:7-8) | `user: User`, `refresh: () => void` | Low |
| [`frontend/src/Widgets/Dispose.svelte`](frontend/src/Widgets/Dispose.svelte:5-6) | `dispose: boolean`, `name: string` | Low |
| [`frontend/src/Widgets/SelectPart.svelte`](frontend/src/Widgets/SelectPart.svelte:8-10) | `type: Type`, `part: number | undefined`, `none = false` | Low |
| [`frontend/src/Widgets/ServiceBadge.svelte`](frontend/src/Widgets/ServiceBadge.svelte:4-5) | `service: {...} | undefined`, `pos = "relative -top-2"` | Low |
| [`frontend/src/Attachment/AttachForm.svelte`](frontend/src/Attachment/AttachForm.svelte:24,28-31) | `part: Part`, `time = new Date()`, `gear`, `hook` | Medium |
| [`frontend/src/Part/Gear.svelte`](frontend/src/Part/Gear.svelte:7) | `params: { id: number }` | Low |
| [`frontend/src/Part/MainCard.svelte`](frontend/src/Part/MainCard.svelte:6) | `part: Part` | Low |
| [`frontend/src/Part/Wizard.svelte`](frontend/src/Part/Wizard.svelte:18-19) | `gear: Part`, `attachees: Attachment[]` | Low |
| [`frontend/src/Shop/Register.svelte`](frontend/src/Shop/Register.svelte:4) | `params: { shopid: number }` | Low |

### Category B: `$:` Reactive Statements → `$derived()` / `$effect()` (9 files)

These files use `$:` reactive declarations. Conversion: Replace with `$derived()` for expressions, `$effect()` for side effects.

| File | `$:` Statements | Conversion |
|------|-----------------|------------|
| [`frontend/src/Part/PartCard.svelte`](frontend/src/Part/PartCard.svelte:34-42) | 4 statements: `list`, `att`, `part`, `dues` | `$derived()` |
| [`frontend/src/Widgets/Plotly.svelte`](frontend/src/Widgets/Plotly.svelte:16) | `redraw(data, layout, config)` | `$effect()` |
| [`frontend/src/Widgets/Actions.svelte`](frontend/src/Widgets/Actions.svelte:57-79) | `actions.set({...})` | `$effect()` |
| [`frontend/src/Usage/UsageChips.svelte`](frontend/src/Usage/UsageChips.svelte:16-18) | 2 statements: `usage` update, `ridesHref` | `$effect()` + `$derived()` |
| [`frontend/src/Widgets/SelectPart.svelte`](frontend/src/Widgets/SelectPart.svelte:12) | `gears = filterValues(...)` | `$derived()` |
| [`frontend/src/Activity/Activities.svelte`](frontend/src/Activity/Activities.svelte:14-37) | Complex `if/else` block for `acts`, `title` | `$derived()` with IIFE |
| [`frontend/src/Widgets/ServiceBadge.svelte`](frontend/src/Widgets/ServiceBadge.svelte:13) | `color = get_color(...)` | `$derived()` |
| [`frontend/src/Part/DisposePart.svelte`](frontend/src/Part/DisposePart.svelte:65) | `action = detach ? ...` | `$derived()` |
| [`frontend/src/Part/Wizard.svelte`](frontend/src/Part/Wizard.svelte:64-66) | `disabled = !groups.reduce(...)` | `$derived()` |

### Category C: Lifecycle Hooks → `$effect()` cleanup (4 files)

These files use `onMount()` and/or `onDestroy()` from Svelte 4. Conversion: Use `$effect()` cleanup function.

| File | Hooks | Notes |
|------|-------|-------|
| [`frontend/src/Widgets/Plotly.svelte`](frontend/src/Widgets/Plotly.svelte:12-14) | `onMount()` | Set `redraw` function |
| [`frontend/src/Header.svelte`](frontend/src/Header.svelte:38-40) | `onDestroy()` | Clear interval cleanup |
| [`frontend/src/Shop/ShopCard.svelte`](frontend/src/Shop/ShopCard.svelte:42-52) | `onMount()`, `onDestroy()` | Event listener management |
| [`frontend/src/Shop/Subscriptions.svelte`](frontend/src/Shop/Subscriptions.svelte:99) | `onMount()` | Load data on mount |
| [`frontend/src/Shop/ShopSubscriptions.svelte`](frontend/src/Shop/ShopSubscriptions.svelte:6) | `onMount()` | Import present, needs conversion |

### Category D: `writable` Store → Module-level pattern (1 file)

| File | Issue | Strategy |
|------|-------|----------|
| [`frontend/src/Widgets/Actions.svelte`](frontend/src/Widgets/Actions.svelte:1-28) | `context="module"` + `writable` store | Convert to module-level `$state` with getter pattern |

### Category E: `tick` import (1 file)

| File | Issue | Notes |
|------|-------|-------|
| [`frontend/src/Activity/TotalsCard.svelte`](frontend/src/Activity/TotalsCard.svelte:27) | `import { tick } from "svelte"` | May already be converted; verify usage |

---

## Detailed File Conversions

### File 1: [`frontend/src/Widgets/Plotly.svelte`](frontend/src/Widgets/Plotly.svelte:1)

**Current syntax**: `onMount()`, `$:`, `export let`
**Changes needed**:
1. Remove `import { onMount } from "svelte"`
2. Convert `export let data;` → add to `$props()` destructuring
3. Convert `export let layout = undefined;` → add to `$props()` with default
4. Convert `$: redraw(data, layout, config);` → `$effect(() => { Plotly.newPlot(plotDiv, data, layout, config); })`
5. Remove `onMount()` block - move logic into `$effect()`

### File 2: [`frontend/src/Widgets/Actions.svelte`](frontend/src/Widgets/Actions.svelte:1)

**Current syntax**: `<script context="module">`, `writable`, `$:`
**Changes needed**:
1. The module-level `actions` store pattern needs special handling
2. Option A: Keep module-level store (export from separate file)
3. Option B: Use Svelte 5's module `$state` with export getter
4. Convert `$: actions.set({...})` → `$effect(() => { ... })`

### File 3: [`frontend/src/Part/PartCard.svelte`](frontend/src/Part/PartCard.svelte:25-42)

**Current syntax**: `export let`, `$:`
**Changes needed**:
1. Convert `export let attachments: Attachment[] = [];` → `let { attachments = [] }: { attachments?: Attachment[] } = $props();`
2. Convert `export let type: Type;` → include in props destructuring
3. Convert `export let children: TreeNode[] = [];` → include in props
4. Convert `export let light = false;` → include in props
5. Convert `$: list = ...` → `let list = $derived(...)`
6. Convert `$: att = ...` → `let att = $derived(...)`
7. Convert `$: part = ...` → `let part = $derived(...)`
8. Convert `$: dues = ...` → `let dues = $derived(...)`

### File 4: [`frontend/src/Widgets/SelectPart.svelte`](frontend/src/Widgets/SelectPart.svelte:8-12)

**Current syntax**: `export let`, `$:`
**Changes needed**:
1. Convert `export let type: Type;` → `$props()`
2. Convert `export let part: number | undefined;` → `$props()` with `$bindable()`
3. Convert `export let none = false;` → `$props()`
4. Convert `$: gears = ...` → `let gears = $derived(...)`

### File 5: [`frontend/src/Widgets/ServiceBadge.svelte`](frontend/src/Widgets/ServiceBadge.svelte:4-13)

**Current syntax**: `export let`, `$:`
**Changes needed**:
1. Convert `export let service: {...} | undefined = undefined;` → `$props()`
2. Convert `export let pos = "relative -top-2";` → `$props()`
3. Convert `$: color = ...` → `let color = $derived(...)`

### File 6: [`frontend/src/Usage/UsageChips.svelte`](frontend/src/Usage/UsageChips.svelte:8-18)

**Current syntax**: `export let`, `$:`
**Changes needed**:
1. Convert all `export let` to `$props()` (note: `usage` needs special handling since it's updated reactively)
2. Convert `$: if (id && $usages[id]) usage = $usages[id];` → `$effect(() => { if (id && $usages[id]) { usage = $usages[id]; } })`
3. Convert `$: ridesHref = ...` → `let ridesHref = $derived(...)`

### File 7: [`frontend/src/Activity/Activities.svelte`](frontend/src/Activity/Activities.svelte:10-37)

**Current syntax**: `export let`, complex `$:` block
**Changes needed**:
1. Convert `export let params: { part: number; start?: number };` → `$props()`
2. The `$:` block computes `acts` and `title` based on `params.part`
3. Convert to: `let { acts, title } = $derived.by(() => { ... })` or use two `$derived` values

### File 8: [`frontend/src/Part/DisposePart.svelte`](frontend/src/Part/DisposePart.svelte:65)

**Current syntax**: `$:`
**Changes needed**:
1. Convert `$: action = detach ? m.action_detach() : m.action_dispose();` → `let action = $derived(detach ? m.action_detach() : m.action_dispose())`
2. Also convert `export const start = ...` - the `start` function uses local variables that need to become `$state()`

### File 9: [`frontend/src/Part/Wizard.svelte`](frontend/src/Part/Wizard.svelte:18-64)

**Current syntax**: `export let`, `$:`
**Changes needed**:
1. Convert `export let gear: Part;` → `$props()`
2. Convert `export let attachees: Attachment[];` → `$props()`
3. Convert `$: disabled = ...` → `let disabled = $derived(...)`
4. Convert local variables (`disabled`) to `$state()` if mutated

### File 10: [`frontend/src/Widgets/Dispose.svelte`](frontend/src/Widgets/Dispose.svelte:5-6)

**Current syntax**: `export let`
**Changes needed**:
1. Convert `export let dispose: boolean;` → `$bindable()` for two-way binding
2. Convert `export let name: string;` → regular `$props()`

### File 11: [`frontend/src/Widgets/Actions.svelte`](frontend/src/Widgets/Actions.svelte:1-28)

**Current syntax**: `<script context="module">`, `writable`
**Changes needed**:
1. Move the `ModalType` type definition to a shared types file or keep in module
2. Convert module-level `writable` to a pattern using `$state()` in a module script
3. Convert the `$: actions.set({...})` to `$effect()`

### File 12-24: Remaining Category A files

These files have simpler `export let` declarations that follow the same pattern as File 10 above.

---

## Header.svelte Lifecycle Conversion

### [`frontend/src/Header.svelte`](frontend/src/Header.svelte:38-40)

**Current syntax**: `onDestroy()`
**Changes needed**:
1. Convert `onDestroy(() => { clearInterval(hook_timer); });` → `$effect(() => { return () => { clearInterval(hook_timer); }; })`

### [`frontend/src/Shop/ShopCard.svelte`](frontend/src/Shop/ShopCard.svelte:42-52)

**Current syntax**: `onMount()`, `onDestroy()`
**Changes needed**:
1. Convert both hooks to single `$effect()` with cleanup:
```typescript
$effect(() => {
  loadPartsCount();
  window.addEventListener("shop-updated", handleShopUpdate as EventListener);
  return () => {
    window.removeEventListener("shop-updated", handleShopUpdate as EventListener);
  };
});
```

---

## Conversion Priority Order

1. **Priority 1 (Simple)**: Files with only `export let` conversions (Category A, Low complexity)
2. **Priority 2 (Expressions)**: Files with `$:` → `$derived()` (Category B, simple cases)
3. **Priority 3 (Effects)**: Files with `$:` side effects → `$effect()` (Category B, complex)
4. **Priority 4 (Lifecycle)**: Files with `onMount`/`onDestroy` → `$effect()` (Category C)
5. **Priority 5 (Complex)**: Files with multiple conversion types (Category D/E)

---

## Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| `actions` module store pattern | High | This is the most complex change; may need architectural decision |
| `$bindable()` two-way binding changes | Medium | Ensure parent components properly use bound values |
| `Plotly.svelte` `@ts-nocheck` | Low | Keep existing TypeScript suppression |
| `Activities.svelte` complex `$:` block | Medium | Carefully preserve reactive logic |
| `DisposePart.svelte` `start` function | Medium | Local state variables need careful migration |

---

## Total Files Requiring Changes

| Category | File Count |
|----------|------------|
| A: `export let` only | 13 |
| B: `$:` reactive statements | 9 |
| C: Lifecycle hooks | 4 |
| D: Module store pattern | 1 |
| E: `tick` import verification | 1 |
| **Total unique files** | **17** |
