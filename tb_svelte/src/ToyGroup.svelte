<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import myfetch, {types, parts, category, catname} from './store.js';
  
  export let params;

  category.set(params.category);
  let promise = myfetch("/part/gear/" + params.category).then(data => parts.set(data))
  onDestroy(() => category.set(undefined));
</script>

{#await promise}
  loading
{:then}
  <div class="row border m-0 p-sm-2">
    {#each $parts as part (part.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear {part} />
      </div>
    {:else}
      You have no {$catname}s to tend 😱
    {/each}
  </div>
{:catch error}
  {error}
{/await}
