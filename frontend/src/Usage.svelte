<script lang="ts">
import {fmtSeconds, fmtNumber} from './store'
import type {Usage} from './types'
 
export let header = false
export let usage: Usage = undefined;

</script>

{#if !header}
  {#if usage}
    <td class="text-right"> 
      {fmtNumber(usage.count)}
    </td>
    <td class="text-right"> 
      {fmtSeconds(usage.time)}
    </td>
    <td class="text-right"> 
      {fmtNumber(Math.round(usage.distance / 1000))}
    </td>
    <td class="text-right"> 
      {fmtNumber(usage.climb)}
    </td>
    <td class="text-right"> 
      {fmtNumber(usage.descend)}
    </td>
  {:else}
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
  {/if}
{:else}
  <th class="text-right" scope="col" title="Number of activities">Count</th>
  <th class="text-right" scope="col" title="Time (h)">Time</th>
  <th class="text-right" scope="col" title="Distance (km)">Distance</th>
  <th class="text-right" scope="col" title="Climb (m)">Climb</th>
  <th class="text-right" scope="col" title="Descend (m)">Descend</th>
{/if}