<script lang="ts">
  import { link } from "svelte-spa-router";
  import { fmtSeconds, fmtNumber } from "../lib/store";
  import { usages, Usage } from "./usage";

  export let header = false;
  export let id: string | undefined = undefined;
  export let usage: Usage = new Usage();
  export let ref: string | number | undefined = undefined;

  $: if (id && $usages[id]) usage = $usages[id];
</script>

{#if !header}
  <td class="text-end">
    {#if ref}
      <a class="text-reset" use:link href={"/activities/" + ref}>
        {fmtNumber(usage.count)}
      </a>
    {:else}
      {fmtNumber(usage.count)}
    {/if}
  </td>
  <td class="text-end">
    {fmtSeconds(usage.time)}
  </td>
  <td class="text-end">
    {fmtNumber(Math.round((usage.distance || 0) / 1000))}
  </td>
  <td class="text-end">
    {fmtNumber(usage.climb)}
  </td>
  <td class="text-end">
    {fmtNumber(usage.descend)}
  </td>
{:else}
  <th class="text-end" scope="col" title="Number of activities">Count</th>
  <th class="text-end" scope="col" title="Time (h)">Time</th>
  <th class="text-end" scope="col" title="Distance (km)">Distance</th>
  <th class="text-end" scope="col" title="Climb (m)">Climb</th>
  <th class="text-end" scope="col" title="Descend (m)">Descend</th>
{/if}
