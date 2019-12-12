<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import NewPart from './NewPart.svelte';
  import {myfetch, filterValues, types, parts, category} from './store.js';
  
  export let params;

  category.set($types[params.category]);
  onDestroy(() => category.set(undefined));
  
  const gears = () => filterValues($parts, (p) => p.what == params.category)
</script>

{#if $category }
  <div class="row border p-sm-2">
    {#each gears() as part (part.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear {part} />
      </div>
    {:else}
      You have no {$category.name} to tend 😱
    {/each}
  </div>
  <div class="container">
      <div class="row p-sm-2">
        <NewPart />
      </div>
  </div>
{:else}
     Error: Category not found!
{/if}