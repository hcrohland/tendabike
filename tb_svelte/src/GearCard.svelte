<script>
  import { slide } from 'svelte/transition';
  import {category, formatSeconds} from './store.js';

  export let part;
  export let display = false;

  export let isOpen = false;
  let showLink = false;

  function model(part) {
    if (part.model =='' && part.vendor == '') {
      return 'unknown model'
    } else {
      return part.vendor + ' '  + part.model
    }
  }
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
     {#if showLink && !display}
          <a href="#/gear/{part.id}" class="badge badge-secondary float-right text-reset">
            &mdash;&GreaterGreater;
          </a>
      {/if}
      
    </div>
  </div>
  {#if isOpen || display}
    <div transition:slide>
      <div class="card-body">
        is a <span class="param">{model(part)}</span>
        purchased <span class="param">{new Date(part.purchase).toLocaleDateString()}</span>
        <br>which you used <span class=param>{part.count.toLocaleString()}</span> times 
        for <span class="param">{formatSeconds(part.time)}</span> hours.
        <p> You covered <span class="param">{parseFloat((part.distance / 1000).toFixed(1)).toLocaleString()}</span> km 
        climbing <span class="param">{part.climb.toLocaleString()}</span> and descending <span class="param">{part.descend.toLocaleString()}</span> meters </p>
        
      </div>
    </div>

  {/if}
</div>
