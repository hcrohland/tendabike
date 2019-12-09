<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import myfetch, {types, parts, category} from './store.js';
  
  export let params;

  category.set($types[params.category]);
  let promise = myfetch("/part/gear/" + params.category)
  onDestroy(() => category.set(undefined));
</script>

<Await {promise} let:data={gears}>
  <div class="row border p-sm-2">
    {#each gears as gear (gear)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear part={$parts[gear]} />
      </div>
    {:else}
      You have no {$category.name}s to tend 😱
    {/each}
  </div>
</Await>
