
<script lang="ts">
import {filterValues, by, types, parts, state, attachments, isAttached, category} from '../store'
import {Button, DropdownItem} from 'sveltestrap'
import Usage from '../Usage.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import NewPart from '../Actions/NewPart.svelte'
import type {Attachment, Part, Type} from '../types'
import ChangePart from '../Actions/ChangePart.svelte';
import Menu from '../Menu.svelte'

export let type: Type;
export let date = new Date;
export let update;
export let attachee: number

let attachPart, newPart, changePart, show_all;

function attachedTo(atts: Attachment[], partId: number, time: Date) {
    let att = filterValues<Attachment>(atts, (x) => x.part_id === partId && isAttached(x, time)).pop()
    if (att == undefined) return
      return $parts[att.gear].name + ' ' + ($types[att.hook].name.split(' ').reverse()[1] || '')
}

function subparts(type: Type, parts) {
  return filterValues<Part>(parts, (p) => p.what == type.id)
            .sort(by("last_used"))
}

</script>

<style>
 .border-2 {
   border-width: 4px;
 }
</style>

<AttachPart bind:attachPart />
<NewPart bind:newPart/>
<ChangePart bind:changePart/>

<tr>
  <th colspan=6 scope="col" class="border-2 text-nowrap"> 
    {type.name}s 
    <button class="btn badge" 
            on:click={() => {show_all = !show_all; update(show_all)}} 
            title={show_all?"hide attached":"show attached"}>
      {#if show_all}
        &#9650;
      {:else}
        &#9660;
      {/if}
    </button>
  </th>
  <th class="border-2 text-nowrap" colspan=80>
    <Button class="badge badge-secondary float-right" on:click={() => newPart(type)}> New {type.name}</Button>
  </th>
</tr>
  {#each subparts(type, $parts).filter((p) => show_all || !attachedTo($attachments, p.id, date))
    as part (part.id)}
  <tr>
    <td class="border-0"></td>
    <td title={part.vendor + ' ' + part.model + ' ' + new Date(part.purchase).toLocaleDateString(navigator.language)}>
      <a href="#/part/{part.id}" class="text-reset">
        {part.name}
      </a>
    </td>
    <Usage {part} />
    {#if attachee > 0 }
      <td>
        {attachedTo($attachments, part.id, date) || '-'}
      </td>
    {/if}
    <td> 
      <Menu>
        <DropdownItem on:click={() => changePart(part)}> Change details </DropdownItem>
        <DropdownItem on:click={() => attachPart(part)}> 
          {attachedTo($attachments, part.id, date)?"Move Part":"Attach Part"}
        </DropdownItem>
      </Menu>
    </td>
  </tr>
{/each}