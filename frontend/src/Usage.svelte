<script lang="ts">
import {fmtSeconds, fmtNumber} from './store'
import type {Attachment, Part} from './types'
 
export let header = false
export let part: Part|Attachment = undefined;

</script>

{#if !header}
  {#if part}
    <td class="text-right"> 
      {fmtNumber(part.count)}
    </td>
    <td class="text-right"> 
      {fmtSeconds(part.time)}
    </td>
    <td class="text-right"> 
      {fmtNumber(Math.round(part.distance / 1000))}
    </td>
    <td class="text-right"> 
      {fmtNumber(part.climb)}
    </td>
    <td class="text-right"> 
      {fmtNumber(part.descend)}
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