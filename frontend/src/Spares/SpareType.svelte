
<script lang="ts">
import {filterValues, by, types, parts, fmtDate, attachments, isAttached} from '../store'
import {Button, DropdownItem} from 'sveltestrap'
import Usage from '../Usage.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import NewPart from '../Actions/NewPart.svelte'
import type {Attachment, Type} from '../types'
import Menu from '../Widgets/Menu.svelte'
import ShowAll from '../Widgets/ShowHist.svelte'

export let type: Type;
export let date = new Date;
export let update;
export let attachee: number

let attachPart, newPart, show_all;

function attachedTo(atts: {[key: string]: Attachment}, partId: number, time: Date) {
    let att = filterValues(atts, (x) => x.part_id === partId && isAttached(x, time)).pop()
    if (att == undefined) return
      return $parts[att.gear].name + ' ' + types[att.hook].prefix
}

$: subparts = filterValues($parts, (p) => p.what == type.id).sort(by("last_used"))
</script>

<style>
 .border-2 {
   border-width: 4px;
 }
</style>

<AttachPart bind:attachPart />
<NewPart bind:newPart/>

<tr>
  <th colspan=6 scope="col" class="border-2 text-nowrap"> 
    {type.name}s 
    {#if subparts.length > 0}
       <ShowAll on:toggle={(e) => {{show_all = e.detail; update(show_all)}}}/>
    {/if}
  </th>
  <th class="border-2 text-nowrap" colspan=80>
    <Button class="badge badge-secondary float-right" on:click={() => newPart(type)}> New {type.name}</Button>
  </th>
</tr>
  {#each subparts.filter((p) => show_all || !(attachedTo($attachments, p.id, date) || p.disposed_at))
    as part (part.id)}
  <tr>
    <td class="border-0"></td>
    <td title={part.vendor + ' ' + part.model + ' ' + fmtDate(part.purchase)}>
      <a href="#/part/{part.id}" 
          style={part.disposed_at ? "text-decoration: line-through;" : ""} 
          class="text-reset">
        {part.name}
      </a>
    </td>
    <Usage usage={part} />
    {#if attachee > 0 }
      <td>
        {#if part.disposed_at}
          disposed {fmtDate(part.disposed_at)}
        {:else}
          {attachedTo($attachments, part.id, date) || '-'}
        {/if}
      </td>
    {/if}
    <td> 
      {#if !part.disposed_at}
      <Menu>
        <DropdownItem on:click={() => attachPart(part)}>
          {#if attachedTo($attachments, part.id, date)} Move {:else} Attach {/if} {type.name}
        </DropdownItem>
      </Menu>
      {/if}
    </td>
  </tr>
{/each}