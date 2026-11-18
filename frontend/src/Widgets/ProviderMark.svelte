<script lang="ts">
  import type { Provider } from "../lib/provider.svelte";

  /// The provider's mark. `colored` (default) is for branded backgrounds
  /// (white Strava logo, white icu badge); `colored={false}` is for light
  /// backgrounds (gray Strava icon, dark icu badge).
  ///
  /// This is a leaf box: it carries no vertical alignment of its own. A mark
  /// next to text sits on the line only when the *container* centers it (an
  /// `items-center` flex row); `vertical-align` cannot center a box taller
  /// than the text's x-height, so it hangs below the line.
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
    class="inline {img}"
  />
{:else}
  <span
    class="inline-flex items-center justify-center {box} rounded font-bold {colored
      ? 'bg-white/20 text-white'
      : 'bg-black/10 text-black dark:bg-white/10 dark:text-white'}">icu</span
  >
{/if}
