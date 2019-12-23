<script>
import {parts} from './store.js'
 
export let part_id = undefined;

let part;
let date = new Date(null); 

$: part = $parts[part_id]


function formatSeconds(sec_num) {
    var hours   = Math.floor(sec_num / 3600);
    var minutes = Math.floor((sec_num - (hours * 3600)) / 60);
    var seconds = sec_num - (hours * 3600) - (minutes * 60);

    if (hours   < 10) {hours   = "0"+hours;}
    if (minutes < 10) {minutes = "0"+minutes;}
    if (seconds < 10) {seconds = "0"+seconds;}
    return hours+':'+minutes+':'+seconds;
}

</script>

{#if part_id}
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