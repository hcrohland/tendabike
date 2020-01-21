<script>
import {myfetch, handleError, filterValues, types, parts, category} from './store.js'
import Usage from './Usage.svelte'
import Await from './Await.svelte'
import Attach from './Attach.svelte'
import NewPart from './NewPart.svelte'

export let params;
  
// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]
category.set(cat);

let spares

let promise; 

update();

function update() {
  console.log("update")
  promise = myfetch('/part/spares/' + cat.id).then((d) => spares = d)
}

</script>

<style>
 .border-2 {
   border-width: 4px;
 }
</style>


{#await promise}
	<Await />
{:then}
    <table class="table table-hover">
      <thead>
        <tr>
          <th scope="col">Part</th>
          <th scope="col">Name</th>
          <Usage />
          <th></th>
        </tr>
      </thead>
      <tbody>
      {#each filterValues($types, (t) => t.main == cat.id && t.id != cat.id) as type}
        <tr>
            <th colspan=8 scope="col" class="border-2 text-nowrap"> {type.name}s <NewPart title='New' cat={type} on:created={update}/></th>
        </tr>
          
         {#each spares
            .map((s) => $parts[s])
            .filter((p) => p.what == type.id)
          as part (part.id)}
          <tr>
           <td class="border-0"></td>
            <td title={part.vendor + ' ' + part.model + ' ' + new Date(part.purchase).toLocaleString()}>{part.name}</td>
            <Usage part_id={part.id} />
            <td> <Attach {part} on:saved={update}/></td>
          </tr>
        {/each}
      {/each}
      </tbody>
    </table>
{:catch error}
  {handleError(error)}
{/await}
