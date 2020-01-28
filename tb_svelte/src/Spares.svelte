<script>
import {filterValues, by, types, parts, attachments, isAttached, category} from './store.js'
import Usage from './Usage.svelte'
import Attach from './Attach.svelte'
import NewPart from './NewPart.svelte'

export let params;
export let date = new Date;

// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]
let show_all = false;
category.set(cat);

let spareTypes = filterValues($types, (t) => t.main == cat.id && t.id != cat.id)

function attachedTo(atts, partId, time) {
    let att = filterValues(atts, (x) => x.part_id === partId && isAttached(x, time))
    return att.pop()
}

function subparts(type, parts) {
  return filterValues(parts, (p) => p.what == type.id)
            .sort(by("purchase"))
}
</script>

<style>
 .border-2 {
   border-width: 4px;
 }
</style>
<span class="badge float-right">
  show attached <input type="checkbox" name="Show all" id="" bind:checked={show_all}>  
</span>
<table class="table table-hover">
  <thead>
    <tr>
      <th scope="col">Part</th>
      <th scope="col">Name</th>
      <Usage header/>
      <td></td>
    </tr>
  </thead>
  <tbody>
  {#each spareTypes as type (type.id)}
    <tr>
        <th colspan=80 scope="col" class="border-2 text-nowrap"> {type.name}s <NewPart title='New' cat={type}/></th>
    </tr>
      {#each subparts(type, $parts).filter((p) => show_all || !attachedTo($attachments, p.id, date))
        as part (part.id)}
      <tr>
        <td class="border-0"></td>
        <td title={part.vendor + ' ' + part.model + ' ' + new Date(part.purchase).toLocaleDateString()}>
          <a href="#/part/{part.id}" class="text-reset">
            {part.name}
          </a>
        </td>
        <Usage {part} />
        <td> <Attach {part}/></td>
      </tr>
    {/each}
  {/each}
  </tbody>
</table>