<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import {myfetch, filterValues, types, parts, category} from './store.js';
  
  export let params;

  category.set($types[params.category]);
  onDestroy(() => category.set(undefined));
  
  const gears = () => filterValues($parts, (p) => p.what === $category.id)
</script>

{#if $category }
  <div class="row border p-sm-2">
    {#each gears() as {id} (id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear part={$parts[id]} />
      </div>
    {:else}
      You have no {$category.name} to tend 😱
    {/each}
  </div>

{:else}
     Error: Category not found!
{/if}