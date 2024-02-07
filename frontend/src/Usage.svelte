<script lang="ts">
  import { link } from "svelte-spa-router";
  import { usages, fmtSeconds, fmtNumber } from "./lib/store";
  import { Usage } from "./lib/types";

  export let id: string | undefined = undefined;
  export let usage: Usage | undefined = undefined;
  export let ref: string | number | undefined = undefined;

  let myusage = new Usage();
  $: if (id) myusage = $usages[id];
  else if (usage) myusage = usage;
</script>

{#if !(usage == undefined && id == undefined)}
  <td class="text-end">
    {#if ref}
      <a class="text-reset" use:link href={"/activities/" + ref}>
        {fmtNumber(myusage.count)}
      </a>
    {:else}
      {fmtNumber(myusage.count)}
    {/if}
  </td>
  <td class="text-end">
    {fmtSeconds(myusage.time)}
  </td>
  <td class="text-end">
    {fmtNumber(Math.round((myusage.distance || 0) / 1000))}
  </td>
  <td class="text-end">
    {fmtNumber(myusage.climb)}
  </td>
  <td class="text-end">
    {fmtNumber(myusage.descend)}
  </td>
{:else}
  <th class="text-end" scope="col" title="Number of activities">Count</th>
  <th class="text-end" scope="col" title="Time (h)">Time</th>
  <th class="text-end" scope="col" title="Distance (km)">Distance</th>
  <th class="text-end" scope="col" title="Climb (m)">Climb</th>
  <th class="text-end" scope="col" title="Descend (m)">Descend</th>
{/if}
