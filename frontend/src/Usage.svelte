<script lang="ts">
  import { fmtSeconds, fmtNumber } from "./lib/store";
  import { Usage } from "./lib/types";

  export let usage: Usage | undefined = undefined;
</script>

{#if usage}
  <td class="text-end">
    {fmtNumber(usage.count)}
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
