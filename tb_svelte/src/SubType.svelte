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
export let hook = false;
void(hook) // get rid of warning...

let show_hist = false; 
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
    {#if i == 0 || show_hist}
      <tr>
        <th scope="row" class="text-nowrap"> 
          {'| '.repeat(level)}
          {#if i == 0 }
            {prefix +  " " + type.name}
            {#if attachments.length > 1}
              <button class="btn" on:click={() => show_hist = !show_hist}>
                {#if show_hist}
                  &#9650;
                {:else}
                  ...
                {/if}
              </button>
            {/if}
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
    {/if}
  {/each}
{/if}
