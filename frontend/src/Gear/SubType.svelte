<script lang="ts">
import {
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownToggle,
  Button
} from 'sveltestrap';
import {parts} from '../store'
import Usage from '../Usage.svelte'
import ReplacePart from '../Actions/ReplacePart.svelte'
import Attach from '../Actions/Attach.svelte'
import Detach from '../Actions/Detach.svelte'
import type {Attachment, Type} from '../types';

export let header = false;
export let attachments: Attachment[] = [];
export let level: number = 0;
export let prefix = "";
export let type: Type | undefined = undefined;
export let hook: Type | undefined = undefined;
void(hook) // get rid of warning...

let isOpen = false;
let show_hist = false; 
let detach, attach, replacepart;
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
            {att.name}
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
          {#if i == 0}
            <Dropdown {isOpen} toggle={() => (isOpen = !isOpen)} size="sm">
              <DropdownToggle caret ></DropdownToggle>
              <DropdownMenu right>
                <DropdownItem on:click={() => replacepart(att)}> replace </DropdownItem>
                <DropdownItem on:click={() => attach($parts[att.part_id])}> move </DropdownItem>
                <DropdownItem on:click={() => detach(att)}> detach </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          
          {:else if $parts[att.part_id]}
            <Button class="float-right" size="sm" on:click={() => attach($parts[att.part_id])}> attach </Button>
          {/if}
        </td>

      </tr>
    {/if}
  {/each}
{/if}
<Attach bind:popup={attach}/> 
<ReplacePart bind:popup={replacepart}/>
<Detach bind:popup={detach}/> 
