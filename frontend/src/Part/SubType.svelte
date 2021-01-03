<script lang="ts">
import {DropdownItem} from 'sveltestrap';
import {isAttached, attTime} from '../store'
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
    <th scope="col">Attached parts</th>
    <th scope="col">Name</th>
    <th scope="col">Attached</th>
    <Usage header/>
    <th></th>
  </tr>    
{:else}
  {#each attachments as att,i (att.idx)}
    {#if i == 0}
      <tr>
        <th scope="row" class="text-nowrap"> 
          {'| '.repeat(level)}
          {prefix +  " " + type.name}
          {#if attachments.length > 1 || (att.part && att.part.count != att.count) }
            <ShowAll bind:show_hist/>
          {/if}
        </th>  
        {#if isAttached(att)}
          <td>
            {#if att.part}
              <a href="#/part/{att.part.id}" 
                style={att.part.disposed_at ? "text-decoration: line-through;" : ""} 
                class="text-reset">
                {att.part.name}
              </a>
            {:else}
              {att.name}
            {/if}
          </td>
          <td> {attTime(att)} </td>  
          <Usage usage={att.part} />
          <td>
            <Menu>
              <DropdownItem on:click={() => attachPart(att.part)}> Move part</DropdownItem>
              <DropdownItem on:click={() => replacePart(att)}> Replace part</DropdownItem>
            </Menu>
          </td>
        {:else}
          <th colspan=80 />
        {/if}
      </tr>   
    {/if}
    {#if show_hist }
      {#if !(i == 0 && att.part && att.part.count == att.count)}
        <tr>
          <th scope="row" class="text-nowrap">
            {'| '.repeat(level+1)}&#9656;
          </th>
          <td>
            {#if att.part}
              <a href="#/part/{att.part.id}" 
                style={att.part.disposed_at ? "text-decoration: line-through;" : ""} 
                class="text-reset">
                {att.part.name}
              </a>
            {:else}
              {att.name}
            {/if}
          </td>
          <td> {attTime(att)} </td>  
          <Usage usage={att} />
          <td>
              <Menu>
                <DropdownItem on:click={() => attachPart(att.part)}>Attach part</DropdownItem>
                <DropdownItem on:click={() => replacePart(att)}> Duplicate part</DropdownItem>
              </Menu>
            </td>
          </tr>
        {/if}
    {/if}
  {/each}
{/if}
<AttachPart bind:attachPart/> 
<ReplacePart bind:replacePart/>
<RecoverPart bind:recoverPart/>
