<!--
	PROTOTYPE — throwaway, do not ship.

	Question: what fill should chips and buttons use?

	Shared prototype component, rendered both by the app route
	(/prototype/chip-fill) and the verify harness (verify/chipfill.html).
	Variants cycle via in-memory state (the floating bar / ← → keys); in the
	app route the choice is also reflected to ?variant= so it is shareable.

	Each variant renders the real card surfaces (from Widgets/color.ts) with
	chips + buttons filled per the variant, so the fill darkness can be judged
	against the actual surfaces in both light and dark mode.

	Convention note: variants B and C use ONE fill for every chip/button
	(midway between the primary and secondary fills), so a chip/button no
	longer takes its card's color for the fill — only cards/Tone keep the
	color prop (for the surface).
-->

<script lang="ts">
  import { goto } from "$app/navigation";
  import { colorClasses } from "../../../Widgets/color.js";

  let { initialVariant = "B" }: { initialVariant?: string } = $props();

  const VARIANTS = {
    A: {
      name: "Current — per-color fills",
      // primary and secondary each keep their own (current) fill
      primaryFill: "bg-primary-700 dark:bg-primary-600",
      secondaryFill: "bg-secondary-700 dark:bg-secondary-600",
    },
    B: {
      name: "Single mid fill (between primary & secondary)",
      // one fill for all chips/buttons: midpoint of the two current fills
      primaryFill: "bg-primary-600 dark:bg-secondary-700",
      secondaryFill: "bg-primary-600 dark:bg-secondary-700",
    },
    C: {
      name: "Single fill, one step lighter",
      primaryFill: "bg-primary-500 dark:bg-secondary-600",
      secondaryFill: "bg-primary-500 dark:bg-secondary-600",
    },
    D: {
      name: "Single fill, adjusted (lighter light / darker dark)",
      // B nudged: light one step lighter (primary-500), dark one step darker
      // (secondary-800)
      primaryFill: "bg-primary-500 dark:bg-secondary-800",
      secondaryFill: "bg-primary-500 dark:bg-secondary-800",
    },
    E: {
      name: "Like D, dark one step darker (secondary-900)",
      // dark = secondary-900, which equals the secondary card surface in dark
      // mode — watch the chip on the secondary card lose contrast
      primaryFill: "bg-primary-500 dark:bg-secondary-900",
      secondaryFill: "bg-primary-500 dark:bg-secondary-900",
    },
    F: {
      name: "FINAL — D light + B dark",
      // light = primary-500 (D), dark = secondary-700 (B): visible on the
      // secondary card in both modes
      primaryFill: "bg-primary-500 dark:bg-secondary-700",
      secondaryFill: "bg-primary-500 dark:bg-secondary-700",
    },
  } as const;

  type VariantKey = keyof typeof VARIANTS;
  const KEYS = Object.keys(VARIANTS) as VariantKey[];

  let dark = $state(false);
  let variant = $state<VariantKey>(
    initialVariant in VARIANTS ? (initialVariant as VariantKey) : "B",
  );

  const v = $derived(VARIANTS[variant]);

  function cycle(dir: 1 | -1) {
    const i = KEYS.indexOf(variant);
    variant = KEYS[(i + dir + KEYS.length) % KEYS.length];
    // Reflect to the URL in the app route (no-op in the harness).
    try {
      const url = new URL(window.location.href);
      url.searchParams.set("variant", variant);
      goto(url.pathname + url.search, { replaceState: true });
    } catch {
      /* harness: no real navigation */
    }
  }

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (
      t.tagName === "INPUT" ||
      t.tagName === "TEXTAREA" ||
      t.isContentEditable
    )
      return;
    if (e.key === "ArrowRight") cycle(1);
    if (e.key === "ArrowLeft") cycle(-1);
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet chip(fill: string, value: string, label: string)}
  <span
    class="inline-flex items-center gap-1 rounded-lg px-3 py-2 shrink-0 {fill}"
  >
    <span class="font-semibold text-sm text-white">{value}</span>
    <span class="font-normal text-xs uppercase tracking-wide text-white">
      {label}
    </span>
  </span>
{/snippet}

<div class={dark ? "dark" : ""}>
  <div
    class="min-h-screen bg-white dark:bg-gray-950 px-4 py-8 max-w-3xl mx-auto pb-24"
  >
    <h1 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-1">
      Chip / button fill prototype
    </h1>
    <p class="text-sm text-gray-600 dark:text-gray-300 mb-6">
      Variant <b>{variant}</b> — {v.name}. Flip with ← → or the bar below;
      toggle light/dark with the last button.
    </p>

    <!-- Primary card (light surface) -->
    <div
      data-testid="primary-surface"
      class={colorClasses.primary.surface + " rounded-xl p-4 mb-4"}
    >
      <div
        class="text-sm font-medium text-secondary-900 dark:text-secondary-100 mb-3"
      >
        Primary card (light surface)
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <span data-testid="primary-chip">
          {@render chip(v.primaryFill, "1,204 km", "odometer")}
        </span>
        <button
          data-testid="primary-button"
          class="rounded-lg px-4 py-2 text-sm font-semibold text-white {v.primaryFill}"
        >
          Add service
        </button>
      </div>
    </div>

    <!-- Secondary card (darker surface) with a nested (flipped) card -->
    <div
      data-testid="secondary-surface"
      class={colorClasses.secondary.surface + " rounded-xl p-4 mb-4"}
    >
      <div
        class="text-sm font-medium text-secondary-900 dark:text-secondary-100 mb-3"
      >
        Secondary card (darker surface)
      </div>
      <div class="flex flex-wrap items-center gap-3 mb-3">
        <span data-testid="secondary-chip">
          {@render chip(v.secondaryFill, "Due in 3 weeks", "chain")}
        </span>
        <button
          data-testid="secondary-button"
          class="rounded-lg px-4 py-2 text-sm font-semibold text-white {v.secondaryFill}"
        >
          Log activity
        </button>
      </div>
      <!-- nested card flips to primary -->
      <div
        data-testid="nested-surface"
        class={colorClasses.primary.surface + " rounded-lg p-3"}
      >
        <div
          class="text-xs font-medium text-secondary-900 dark:text-secondary-100 mb-2"
        >
          Nested card (flips to primary)
        </div>
        <span data-testid="nested-chip">
          {@render chip(v.primaryFill, "42 rpm", "cadence")}
        </span>
      </div>
    </div>

    <!-- Page base (no enclosing card) -->
    <div
      class="rounded-xl border border-dashed border-gray-300 dark:border-gray-700 p-4"
    >
      <div class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
        Page base (no card) — standalone chip + button
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <span data-testid="base-chip">
          {@render chip(v.secondaryFill, "3", "open items")}
        </span>
        <button
          data-testid="base-button"
          class="rounded-lg px-4 py-2 text-sm font-semibold text-white {v.secondaryFill}"
        >
          View all
        </button>
      </div>
    </div>
  </div>

  <!-- Floating prototype switcher (not part of the design under evaluation) -->
  <div
    class="fixed bottom-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 rounded-full bg-gray-900 text-white px-2 py-1.5 shadow-lg"
  >
    <button
      data-testid="prev"
      onclick={() => cycle(-1)}
      aria-label="Previous variant"
      class="h-8 w-8 grid place-items-center rounded-full hover:bg-gray-700"
    >
      ←
    </button>
    <span class="text-sm font-medium min-w-56 text-center">
      {variant} — {v.name}
    </span>
    <button
      data-testid="next"
      onclick={() => cycle(1)}
      aria-label="Next variant"
      class="h-8 w-8 grid place-items-center rounded-full hover:bg-gray-700"
    >
      →
    </button>
    <button
      data-testid="toggle-dark"
      onclick={() => (dark = !dark)}
      aria-label="Toggle light/dark"
      class="h-8 w-8 grid place-items-center rounded-full hover:bg-gray-700"
    >
      {dark ? "☀" : "☾"}
    </button>
  </div>
</div>
