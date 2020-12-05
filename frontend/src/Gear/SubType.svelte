<script lang="ts">
import {DropdownItem} from 'sveltestrap';
import {parts, isAttached, fmtDate} from '../store'
import Usage from '../Usage.svelte'
import ReplacePart from '../Actions/ReplacePart.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import DetachPart from '../Actions/DetachPart.svelte'
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
let detachPart, attachPart, replacePart, changePart;
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
          <a href="#/part/{att.part_id}" 
            style={$parts[att.part_id].disposed_at ? "text-decoration: line-through;" : ""} 
            class="text-reset">
            {$parts[att.part_id].name}
          </a>
        {:else}
          {att.name}
        {/if}
        </td>
        <td class="text-right"> {fmtDate(att.attached)} 
          {#if att.detached}
            -
            {fmtDate(att.detached)}
          {/if}
        </td><Usage part={$parts[att.part_id] || att} />
        <td>
          {#if !$parts[att.part_id].disposed_at}
          <Menu>
            {#if isAttached(att, new Date)}
            <DropdownItem on:click={() => replacePart(att)}> Replace part</DropdownItem>
            <DropdownItem on:click={() => attachPart($parts[att.part_id])}> Move part</DropdownItem>
            <DropdownItem on:click={() => detachPart(att)}> Detach part</DropdownItem>
            {:else}
            <DropdownItem on:click={() => attachPart($parts[att.part_id])}> Attach part</DropdownItem>
            {/if}
            <DropdownItem on:click={() => changePart($parts[att.part_id])}> Change details </DropdownItem>
          </Menu>
          {/if}
        </td>
      </tr>
    {/if}
  {/each}
{/if}
<ChangePart bind:changePart/> 
<AttachPart bind:attachPart/> 
<ReplacePart bind:replacePart/>
<DetachPart bind:detachPart/> 
