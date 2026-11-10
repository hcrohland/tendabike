<script lang="ts">
  import type { Provider } from "../lib/provider";

  /// The provider's mark on a branded (accent-colored) background:
  /// Strava's white logo, intervals.icu's "icu" badge.
  let {
    provider,
    size = "md",
  }: { provider: Provider; size?: "sm" | "md" | "lg" } = $props();

  let box = $derived(
    size === "sm"
      ? "h-4 min-w-4 text-[9px]"
      : size === "lg"
        ? "h-6 min-w-6 px-1.5 text-[11px]"
        : "h-5 min-w-5 text-[10px]",
  );
  let img = $derived(
    size === "sm" ? "h-4 w-4" : size === "lg" ? "h-7 w-7" : "h-6 w-6",
  );
</script>

{#if provider === "strava"}
  <img src="/strava.png" alt="" class="inline {img} align-middle" />
{:else}
  <span
    class="inline-flex items-center justify-center {box} rounded bg-white/20 text-white font-bold align-middle"
    >icu</span
  >
{/if}
