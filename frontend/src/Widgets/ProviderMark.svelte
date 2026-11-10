<script lang="ts">
  import type { Provider } from "../lib/provider";

  /// The provider's mark. `colored` (default) is for branded backgrounds
  /// (white Strava logo, white icu badge); `colored={false}` is for light
  /// backgrounds (gray Strava icon, dark icu badge).
  let {
    provider,
    size = "md",
    colored = false,
  }: {
    provider: Provider;
    size?: "sm" | "md" | "lg";
    colored?: boolean;
  } = $props();

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
  <img
    src={colored ? "/strava.png" : "/strava_grey.png"}
    alt=""
    class="inline {img} -translate-y-px"
  />
{:else}
  <span
    class="inline-flex items-center justify-center {box} rounded font-bold -translate-y-px {colored
      ? 'bg-white/20 text-white'
      : 'bg-black/10 text-black'}">icu</span
  >
{/if}
