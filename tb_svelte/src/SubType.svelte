<script>
import {parts, by} from './store.js'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'
import PartHist from './PartHist.svelte'

export let header = false;
export let attachments = undefined;
export let level = undefined;
export let prefix = "";
export let type = "";
export const hook = "";

</script>

  {#if header}
      <tr>
        <th scope="col">Part</th>
        <th scope="col">Name</th>
        <th scope="col" class="text-right">Attached</th>
        <Usage header/>
        <th></th>
      </tr>    
  {:else}
      {#each attachments as att,i (att.attached)}
        <tr>
          <th scope="row" class="text-nowrap"> 
            {'| '.repeat(level)}
            {#if i == 0}
              {prefix +  " " + type.name}
            {:else}
              |
            {/if}
          </th>
          <td>
          {#if $parts[att.part_id]}
            <a href="#/part/{att.part_id}" class="text-reset">
              {att.name}
            </a>  
          {:else}
            {att.name}
          {/if}
          </td>
          <td class="text-right"> {new Date(att.attached).toLocaleDateString()} </td >
          <Usage part={$parts[att.part_id] || att} />
        </tr>
      {/each}
  {/if}
