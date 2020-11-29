<script lang="ts">
import {filterValues, by, types, parts, state, attachments, isAttached, category} from './store'
import {Button, DropdownItem} from 'sveltestrap'
import Usage from './Usage.svelte'
import Attach from './Actions/Attach.svelte'
import NewPart from './Actions/NewPart.svelte'
import type {Attachment, Part, Type} from './types'
import ChangePart from './Actions/ChangePart.svelte';
import Menu from './Menu.svelte'

export let params;
export let date = new Date;

let attach, newPart, changePart;

// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]

category.set(cat);

let spareTypes = filterValues<Type>($types, (t) => t.main == cat.id && t.id != cat.id)

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

<Attach bind:popup={attach} />
<NewPart bind:popup={newPart}/>
<ChangePart bind:popup={changePart}/>

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
      <th>
        <span class="badge float-right">
          show attached <input type="checkbox" name="Show all" id="" bind:checked={$state.show_all_spares}>  
        </span>
      </th>
    </tr>
  </thead>
  <tbody>
  {#each spareTypes as type (type.id)}
    <tr>
      <th colspan=80 scope="col" class="border-2 text-nowrap"> {type.name}s 
          <Button class="badge badge-secondary float-right" on:click={() => newPart(type)}> New {type.name}</Button>
      </th>
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
          <Menu>
            <DropdownItem on:click={() => changePart(part)}> Change details </DropdownItem>
            <DropdownItem on:click={() => attach(part)}> 
              {attachedTo($attachments, part.id, date)?"Move Part":"Attach Part"}
            </DropdownItem>
          </Menu>
        </td>
      </tr>
    {/each}
  {/each}
  </tbody>
</table>