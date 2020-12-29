<script lang="ts">
import {DropdownItem} from 'sveltestrap';
import {parts, isAttached, fmtDate, maxDate} from '../store'
import Usage from '../Usage.svelte'
import RecoverPart from '../Actions/RecoverPart.svelte'
import ReplacePart from '../Actions/ReplacePart.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import type {Attachment, Type} from '../types';
import Menu from '../Widgets/Menu.svelte';
import ShowAll from '../Widgets/ShowHist.svelte';

export let header = false;
export let attachments: Attachment[] = [];
export let level: number = 0;
export let prefix = "";
export let type: Type | undefined = undefined;
export let hook: Type | undefined = undefined;
void(hook) // get rid of warning...

let show_hist = false; 
let attachPart, replacePart, recoverPart;

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
    {#if i == 0 && !isAttached(att)}
    <tr>
      <th scope="row" class="text-nowrap"> 
        {'| '.repeat(level)}
          {prefix +  " " + type.name}
          <ShowAll bind:show_hist/>
      </th>
      <th colspan=80>
      </th>
    </tr>
    {/if}
    {#if show_hist || ( i == 0 && isAttached(att) ) }
      <tr>
        <th scope="row" class="text-nowrap"> 
          {'| '.repeat(level)}
          {#if i == 0 && isAttached(att) }
            {prefix +  " " + type.name}
            {#if attachments.length > 1}
              <ShowAll bind:show_hist/>
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
          {#if att.detached < maxDate}
            -
            {fmtDate(att.detached)}
          {/if}
        </td><Usage usage={$parts[att.part_id] || att} />
        <td>
          {#if $parts[att.part_id] && !$parts[att.part_id].disposed_at}
            <Menu>
              {#if isAttached(att, new Date)}
              <DropdownItem on:click={() => attachPart($parts[att.part_id])}> Move part</DropdownItem>
              {:else}
              <DropdownItem on:click={() => attachPart($parts[att.part_id])}> Attach part</DropdownItem>
              {/if}
              {#if i == 0}
              <DropdownItem on:click={() => replacePart(att)}> Replace part</DropdownItem>
              {/if}
            </Menu>
          {/if}
        </td>
      </tr>
    {/if}
  {/each}
{/if}
<AttachPart bind:attachPart/> 
<ReplacePart bind:replacePart/>
<RecoverPart bind:recoverPart/>
