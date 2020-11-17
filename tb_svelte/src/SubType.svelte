<script lang="ts">
import {Button} from 'sveltestrap';
import {parts} from './store'
import Usage from './Usage.svelte'
import ReplacePart from './ReplacePart.svelte'
import Attach from './Attach.svelte'
import type {Attachment, Type} from './types';
import type { replace } from 'svelte-spa-router';

export let header = false;
export let attachments: Attachment[] = [];
export let level: number;
export let prefix = "";
export let type: Type;
export let hook: Type;
void(hook) // get rid of warning...

let show_hist = false; 
let attach, replacepart;
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
  {#each attachments as att,i (att.part_id + "/" + att.attached)}
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
        <td class="text-right"> {new Date(att.attached).toLocaleDateString(navigator.language)} </td >
        <Usage part={$parts[att.part_id] || att} />
        <td>
          {#if i == 0}
            <Button class="badge badge-secondary float-right" on:click={() => replacepart(att)}> replace </Button>
          {:else if $parts[att.part_id]}
            <Button class="badge badge-secondary float-right" on:click={() => attach($parts[att.part_id])}> attach </Button>
          {/if}
        </td>

      </tr>
    {/if}
  {/each}
{/if}
<Attach bind:popup={attach}/> 
<ReplacePart bind:popup={replacepart}/>
