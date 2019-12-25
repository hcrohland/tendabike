<script>
  import { slide } from 'svelte/transition';

  export let part;

  let isOpen = false;
  let showLink = false;
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
    <div class="card-header" on:click={() => (isOpen = !isOpen)} on:mouseenter={()=> showLink = true} on:mouseleave={()=> showLink = false}>
      <span class="h5 mb-0"> 
        {part.name} 
      </span>
     {#if showLink}
          <a href="#/part/{part.id}" type="button" class="float-right">
            &mdash;&GreaterGreater;
          </a>
      {/if}
      
    </div>
  </div>
  {#if isOpen}
    <div transition:slide>
      <div class="card-body">
        is a <span class="param">{part.vendor} {part.model}</span> 
        which you used <span class=param>{part.count.toLocaleString()}</span> times 
        for <span class="param">{Math.floor(part.time /3600).toLocaleString()}:{String(Math.floor(part.time/60)%60).padStart(2, '0')
}</span> hours
        <p> You covered <span class="param">{parseFloat((part.distance / 1000).toFixed(1)).toLocaleString()}</span> km 
        climbing <span class="param">{part.climb.toLocaleString()}</span> and descending <span class="param">{part.descend.toLocaleString()}</span> meters </p>
        
      </div>
    </div>

  {/if}
</div>
