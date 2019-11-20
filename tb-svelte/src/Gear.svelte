<script>
  import { slide } from 'svelte/transition';

  export let part;

  let isOpen = false;
</script>

<style>
  .header:hover {
    background-color: lightgray;
  }

  .param {
      font-weight: bold;
  }
</style>

<div class="card">
  <div class="header">
    <div class="card-header" on:click={() => (isOpen = !isOpen)}>
      <h5> {part.name} </h5>
    </div>
  </div>
  {#if isOpen}
    <div transition:slide>
      <div class="card-body">
        is a <span class="param">{part.vendor} {part.model}</span> 
        which you used <span class=param>{part.count}</span> times 
        for <span class="param">{Math.floor(part.time /3600)}:{Math.floor(part.time/60)%60}</span> hours
        <p> You covered <span class="param">{(part.distance / 1000).toFixed(1)}</span> km 
        climbing <span class="param">{part.climb}</span> and descending <span class="param">{part.descend}</span> meters </p>
      </div>
    </div>
  {/if}
</div>
