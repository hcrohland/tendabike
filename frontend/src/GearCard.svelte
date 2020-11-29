<script lang="ts">
  import { slide } from 'svelte/transition';
  import {Card, CardBody, CardHeader} from 'sveltestrap';
  import {category, formatSeconds} from './store';

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

<Card>
  <div class="header" on:mouseenter={()=> showLink = true} on:mouseleave={()=> showLink = false}>
    <CardHeader on:click={() => (isOpen = !isOpen)} >
      <span class="h5 mb-0"> 
        {part.name} 
      </span>
      {#if showLink && !display}
        <div class="float-right text-reset">

          <a href={"/strava/bikes/" + part.id} alt="View on Strava" target="_blank"><img src="strava.png" alt="View on Strava" title="View on Strava" > </a> 
          <a href="#/gear/{part.id}" class="badge badge-secondary" title={"View "+ $category.name.toLowerCase() + " details"}>
            &Longrightarrow;
          </a>
        </div>
      {/if}
    </CardHeader>
  </div>
  {#if isOpen || display}
    <div transition:slide>
      <CardBody>
        is a <span class="param">{model(part)}</span>
        purchased <span class="param">{new Date(part.purchase).toLocaleDateString(navigator.language)}</span>
        <br>which you used <span class=param>{part.count.toLocaleString(navigator.language)}</span> times 
        for <span class="param">{formatSeconds(part.time)}</span> hours.
        <p> You covered <span class="param">{parseFloat((part.distance / 1000).toFixed(1)).toLocaleString(navigator.language)}</span> km 
        climbing <span class="param">{part.climb.toLocaleString(navigator.language)}</span> and descending <span class="param">{part.descend.toLocaleString(navigator.language)}</span> meters </p>
        
      </CardBody>
    </div>

  {/if}
</Card>
