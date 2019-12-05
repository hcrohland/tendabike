<script>
  import { onDestroy } from 'svelte';
  import Gear from './Gear.svelte';
  import myfetch, {types, parts, category} from './store.js';
  
  export let params;

  category.set(params.category)
  parts.set(new Object)
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
      You have no {$types[params.category].name }s to tend 😱
    {/each}
  </div>
{:catch error}
  {error}
{/await}
