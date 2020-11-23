<script lang="ts">
import {parts, formatSeconds} from './store'
 
export let header = false
export let part = undefined;

</script>

{#if !header}
  {#if part}
    <td class="text-right"> 
      {part.count.toLocaleString(navigator.language)}
    </td>
    <td class="text-right"> 
      {formatSeconds(part.time)}
    </td>
    <td class="text-right"> 
      {Math.round(part.distance / 1000).toLocaleString(navigator.language)}
    </td>
    <td class="text-right"> 
      {part.climb.toLocaleString(navigator.language)}
    </td>
    <td class="text-right"> 
      {part.descend.toLocaleString(navigator.language)}
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