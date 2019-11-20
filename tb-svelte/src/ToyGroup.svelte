<script>
  import Gear from './Gear.svelte';
  
  export let parts;
</script>

<div class="row border m-0 p-sm-2">
  {#each parts as part (part.id)}
    <div class="col-md-6 p-0 p-sm-2">
      <Gear {part} />
    </div>
  {/each}
</div>
