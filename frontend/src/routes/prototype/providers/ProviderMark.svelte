<!-- PROTOTYPE (throwaway): a tiny per-provider mark. Strava reuses the real
     asset; intervals.icu has no logo here, so it gets a plain text badge. -->
<script lang="ts">
  import type { Provider } from "./mock";

  let {
    provider,
    size = "md",
  }: { provider: Provider; size?: "sm" | "md" | "lg" } = $props();

  let box = $derived(
    size === "sm"
      ? "h-4 min-w-4 text-[9px]"
      : size === "lg"
        ? "h-6 min-w-6 text-[11px]"
        : "h-5 min-w-5 text-[10px]",
  );
  let img = $derived(
    size === "sm" ? "h-4 w-4" : size === "lg" ? "h-6 w-6" : "h-5 w-5",
  );
</script>

{#if provider === "strava"}
  <img
    src="/strava_grey.png"
    alt="Strava"
    class="inline {img} ml-1 align-middle"
  />
{:else}
  <span
    class="inline-flex items-center justify-center {box} ml-1 rounded bg-neutral-800 dark:bg-neutral-200 dark:text-neutral-900 text-white font-bold align-middle"
    >icu</span
  >
{/if}
