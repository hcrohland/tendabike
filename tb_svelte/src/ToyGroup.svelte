<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import Await from './Await.svelte';
  import myfetch, {types, parts, category} from './store.js';
  
  export let params;

  category.set($types[params.category]);
  
  $:  gears = Object.values($parts).filter(
          (p) => { return p.what.toString() === params.category}
        );

  onDestroy(() => category.set(undefined));
</script>

  <div class="row border p-sm-2">
    {#each gears as gear (gear.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear part={$parts[gear.id]} />
      </div>
    {:else}
      You have no {$category.name}s to tend 😱
    {/each}
  </div>
