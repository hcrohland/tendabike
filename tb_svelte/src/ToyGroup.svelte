<script>
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import NewPart from './NewPart.svelte';
  import {filterValues, types, parts, category} from './store.js';
  
  export let params;
  
  // Cannot use category directly since it 
  // is unset during destroy and the router gets confused
  let cat = $types[params.category]
  category.set(cat);
  
  const gears = () => filterValues($parts, (p) => p.what == params.category)
</script>

{#if cat }
  <div class="row border p-sm-2">
    {#each gears() as {id} (id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear {id} />
      </div>
    {:else}
      You have no {cat.name} to tend 😱
    {/each}
  </div>
  <div class="container">
      <div class="row p-sm-2">
        <NewPart {cat}/>
      </div>
  </div>
{:else}
     Error: Category not found!
{/if}