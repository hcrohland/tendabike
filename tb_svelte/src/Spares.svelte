<script lang="ts">
import {filterValues, by, types, parts, state, attachments, isAttached, category} from './store'
import {Button} from 'sveltestrap'
import Usage from './Usage.svelte'
import Attach from './Actions/Attach.svelte'
import NewPart from './Actions/NewPart.svelte'
import type {Attachment, Part, Type} from './types'

export let params;
export let date = new Date;

// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]

category.set(cat);

let spareTypes = filterValues<Type>($types, (t) => t.main == cat.id && t.id != cat.id)

function attachedTo(atts: Attachment[], partId: number, time: Date) {
    let att = filterValues<Attachment>(atts, (x) => x.part_id === partId && isAttached(x, time)).pop()
    if (att == undefined) return
      return $parts[att.gear].name + ' ' + ($types[att.what].name.split(' ').reverse()[1] || '')
}

function subparts(type: Type, parts) {
  return filterValues<Part>(parts, (p) => p.what == type.id)
            .sort(by("last_used"))
}

let attach: (newpart: Part) => void;
let newpart: (type: Type) => void;
</script>

<style>
 .border-2 {
   border-width: 4px;
 }
</style>

<Attach bind:popup={attach} />
<NewPart bind:popup={newpart}/>

<table class="table table-hover">
  <thead>
    <tr>
      <th scope="col">Part</th>
      <th scope="col">Name</th>
      <Usage header/>
      {#if $state.show_all_spares}
        <th>
          Attached to
        </th>
      {/if}
      <td>
        <span class="badge float-right">
          show attached <input type="checkbox" name="Show all" id="" bind:checked={$state.show_all_spares}>  
        </span>
      </td>
    </tr>
  </thead>
  <tbody>
  {#each spareTypes as type (type.id)}
    <tr>
      <th colspan=80 scope="col" class="border-2 text-nowrap"> {type.name}s 
          <Button class="badge badge-secondary float-right" on:click={() => newpart(type)}> New {type.name}</Button>
    </tr>
      {#each subparts(type, $parts).filter((p) => $state.show_all_spares || !attachedTo($attachments, p.id, date))
        as part (part.id)}
      <tr>
        <td class="border-0"></td>
        <td title={part.vendor + ' ' + part.model + ' ' + new Date(part.purchase).toLocaleDateString(navigator.language)}>
          <a href="#/part/{part.id}" class="text-reset">
            {part.name}
          </a>
        </td>
        <Usage {part} />
        {#if $state.show_all_spares}
          <td>
            {attachedTo($attachments, part.id, date) || '-'}
          </td>
        {/if}
        <td> 
          <span type="button" class="badge badge-secondary float-right" on:click={() => attach(part)}>
            {attachedTo($attachments, part.id, date)?"move":"attach"}
          </span>
        </td>
      </tr>
    {/each}
  {/each}
  </tbody>
</table>