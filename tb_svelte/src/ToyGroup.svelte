<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import myfetch, {types, parts, category} from './store.js';
  
  export let params;

  category.set($types[params.category]);
  let promise = myfetch("/part/gear/" + params.category).then(data => parts.setMap(data))
  onDestroy(() => category.set(undefined));
</script>

<Await {promise}>
  <div class="row border p-sm-2">
    {#each Object.values($parts) as part (part.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear {part} />
      </div>
    {:else}
      You have no {$category.name}s to tend 😱
    {/each}
  </div>
</Await>
