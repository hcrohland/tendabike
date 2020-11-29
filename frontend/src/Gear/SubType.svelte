<script lang="ts">
import {DropdownItem} from 'sveltestrap';
import {parts} from '../store'
import Usage from '../Usage.svelte'
import ReplacePart from '../Actions/ReplacePart.svelte'
import Attach from '../Actions/Attach.svelte'
import Detach from '../Actions/Detach.svelte'
import type {Attachment, Type} from '../types';
import ChangePart from '../Actions/ChangePart.svelte';
import Menu from '../Menu.svelte';

export let header = false;
export let attachments: Attachment[] = [];
export let level: number = 0;
export let prefix = "";
export let type: Type | undefined = undefined;
export let hook: Type | undefined = undefined;
void(hook) // get rid of warning...

let show_hist = false; 
let detach, attach, replacePart, changePart;
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
              <button class="btn badge" on:click={() => show_hist = !show_hist}>
                {#if show_hist}
                  &#9650;
                {:else}
                  &#9660;
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
            {$parts[att.part_id].name}
          </a>
        {:else}
          {att.name}
        {/if}
        </td>
        <td class="text-right"> {new Date(att.attached).toLocaleDateString(navigator.language)} 
          {#if att.detached}
            -
            {new Date(att.detached).toLocaleDateString(navigator.language)}
          {/if}
        </td><Usage part={$parts[att.part_id] || att} />
        <td>
          <Menu>
            <DropdownItem on:click={() => changePart($parts[att.part_id])}> Change details </DropdownItem>
            <DropdownItem on:click={() => attach($parts[att.part_id])}> Move part</DropdownItem>
            {#if i == 0}
              <DropdownItem on:click={() => replacePart(att)}> Replace part</DropdownItem>
              <DropdownItem on:click={() => detach(att)}> Detach part</DropdownItem>
            {/if}
          </Menu>
        </td>
      </tr>
    {/if}
  {/each}
{/if}
<ChangePart bind:popup={changePart}/> 
<Attach bind:popup={attach}/> 
<ReplacePart bind:popup={replacePart}/>
<Detach bind:popup={detach}/> 
