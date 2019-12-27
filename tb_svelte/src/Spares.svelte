<script>
import {myfetch, types, parts, category} from './store.js'
import Usage from './Usage.svelte'
import Await from './Await.svelte'
import NewPart from './NewPart.svelte'

export let params;
  
// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]
category.set(cat);

let spares

let promise; 
promise = myfetch('/part/spares/' + cat.id) 
</script>


<Await {promise} let:data={spares}>
  {#if spares.length > 0}
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
        {#each spares
            .map((s) => $parts[s])
            .sort((a,b) => $types[a.what].order > $types[b.what].order)
          as part (part.id)}
          <tr>
            <th scope="row" class="text-nowrap"> {$types[part.what].name} </th>
            <td>{part.name}</td>
            <Usage part_id={part.id} />
            <th> <NewPart title='New' cat={$types[part.what]}/></th>
          </tr>
        {/each}
      </tbody>
    </table>

  {:else}
    You have no {cat.name} spares!
  {/if}
</Await>