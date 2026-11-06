<!-- PROTOTYPE (throwaway) route — issue #752 "Frontend provider UX".
     Three variants of the two-provider frontend, switchable via ?variant=.
     Sample data, no backend calls. Delete this whole /prototype/ tree when
     the decision is captured. -->
<script lang="ts">
  import { page } from "$app/state";
  import VariantA from "./VariantA.svelte";
  import VariantB from "./VariantB.svelte";
  import VariantC from "./VariantC.svelte";
  import VariantD from "./VariantD.svelte";
  import PrototypeSwitcher from "./PrototypeSwitcher.svelte";

  const variants = ["A", "B", "C", "D"];
  const names: Record<string, string> = {
    A: "Inline choice",
    B: "Neutral + picker",
    C: "Provider identity",
    D: "Assembled (winner)",
  };
  let variant = $derived(page.url.searchParams.get("variant") ?? "D");

  const taglines: Record<string, string> = {
    A: "A second peer button appears wherever Strava does; small per-provider icons; both logos in the footer.",
    B: "One neutral entry opens a picker; the connected app stays provider-quiet (monochrome dots, neutral footnote).",
    C: "The provider is a first-class identity: hero cards, branded onboarding, prominent badges, partner headline.",
    D: "The picked combination: C's hero-card connect + branded onboarding (+ header Connect button), A's small inline view-on icons, B's neutral-footnote branding.",
  };
</script>

<div class="max-w-3xl mx-auto pb-24">
  <div
    class="rounded-lg bg-amber-100 dark:bg-amber-900/30 border border-amber-300 dark:border-amber-700 text-amber-900 dark:text-amber-100 p-3 text-sm mb-4"
  >
    <strong>PROTOTYPE</strong> — throwaway mockup for the two-provider frontend (issue
    #752). Sample data, no backend calls. Flip variants with the bar below (or ← /
    →).
  </div>

  <h1 class="text-xl font-bold mb-1">
    Provider UX — variant {variant}: {names[variant]}
  </h1>
  <p class="text-sm text-text-1 mb-4">{taglines[variant]}</p>

  {#if variant === "A"}
    <VariantA />
  {:else if variant === "B"}
    <VariantB />
  {:else if variant === "C"}
    <VariantC />
  {:else}
    <VariantD />
  {/if}
</div>

<PrototypeSwitcher {variants} {names} />
