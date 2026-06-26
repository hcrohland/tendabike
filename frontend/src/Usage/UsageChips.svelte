<script lang="ts">
  import { link } from "svelte-spa-router";
  import { fmtNumber, fmtSeconds } from "../lib/store";
  import { Usage, usages } from "../lib/usage";

  export let id: string | undefined = undefined;
  export let usage: Usage = new Usage();
  export let ref: string | number | undefined = undefined;

  $: if (id && $usages[id]) usage = $usages[id];
</script>

<div class="grid grid-cols-3 gap-2 mt-2">
  <div
    class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
  >
    {#if ref}
      <a
        class="flex items-center gap-1 text-reset"
        use:link
        href={"/activities/" + ref}
      >
        <span class="font-semibold text-sm">{fmtNumber(usage.count)}</span>
        <span
          class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
          >rides</span
        >
      </a>
    {:else}
      <span class="font-semibold text-sm">{fmtNumber(usage.count)}</span>
      <span
        class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
        >rides</span
      >
    {/if}
  </div>
  <div
    class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
  >
    <span class="font-semibold text-sm">{fmtSeconds(usage.time)}</span>
    <span
      class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
      >h</span
    >
  </div>
  <div
    class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
  >
    <span class="font-semibold text-sm"
      >{fmtNumber(Math.round((usage.distance || 0) / 1000))}</span
    >
    <span
      class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
      >km</span
    >
  </div>
  <div
    class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
  >
    <span class="font-semibold text-sm">{fmtNumber(usage.climb)}</span>
    <span
      class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
      >↑m</span
    >
  </div>
  <div
    class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
  >
    <span class="font-semibold text-sm">{fmtNumber(usage.descend)}</span>
    <span
      class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
      >↓m</span
    >
  </div>
  {#if usage.energy > 0}
    <div
      class="flex items-center gap-1 px-3 py-2 rounded-lg bg-gray-100 dark:bg-gray-700 shrink-0"
    >
      <span class="font-semibold text-sm">{fmtNumber(usage.energy)}</span>
      <span
        class="font-normal text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400"
        >kJ</span
      >
    </div>
  {/if}
</div>
