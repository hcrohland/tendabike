<script>
import {parts, formatSeconds} from './store.js'
 
export let header = false
export let part = undefined;

</script>

{#if !header}
  {#if part}
    <td class="text-right"> 
      {part.count.toLocaleString()}
    </td>
    <td class="text-right"> 
      {formatSeconds(part.time)}
    </td>
    <td class="text-right"> 
      {Math.round(part.distance / 1000).toLocaleString()}
    </td>
    <td class="text-right"> 
      {part.climb.toLocaleString()}
    </td>
    <td class="text-right"> 
      {part.descend.toLocaleString()}
    </td>
  {:else}
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
    <td class="text-right">N/A</td>
  {/if}
{:else}
  <th class="text-right" scope="col">Count</th>
  <th class="text-right" scope="col">Time</th>
  <th class="text-right" scope="col">km</th>
  <th class="text-right" scope="col">climb</th>
  <th class="text-right" scope="col">descend</th>
{/if}