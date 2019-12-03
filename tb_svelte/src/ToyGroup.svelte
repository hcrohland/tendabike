<script>
  import Gear from './Gear.svelte';
  import myfetch, {types} from './store.js';
  
  export let params;

  let promise = myfetch("/part/gear/" + params.category)

</script>

{#await promise}
  loading
{:then parts}
  <div class="row border m-0 p-sm-2">
    {#each parts as part (part.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <Gear {part} />
      </div>
    {:else}
      You have not {$types[params.category].name }s to tend 😱
    {/each}
  </div>
{:catch error}
  {error}
{/await}
