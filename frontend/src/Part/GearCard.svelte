<script lang="ts">
  // import { slide } from 'svelte/transition';
  import {Card, CardBody, CardHeader, Row, Col} from 'sveltestrap';
  import {types, fmtSeconds, fmtDate, fmtNumber} from '../store';
  import type {Part} from '../types'

  export let part: Part;
  export let display = false;

  let isOpen = false;
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
    <CardHeader class="h5 mb-0" on:click={() => (isOpen = !isOpen)} >
      <Row>
        <Col>
          {part.name}
        </Col>
        {#if showLink || display}
        <Col>
          <slot />
        </Col>
        {/if}
      </Row>
    </CardHeader>
  </div>
  {#if isOpen || display}
    <!-- conflict with spa-router -->
    <!-- <div transition:slide> -->
      <CardBody>
        is a <span class="param">{model(part)}</span>
        {#if part.what == 1}
           <a href={"/strava/bikes/" + part.id} alt="View on Strava" target="_blank"><img src="strava_grey.png" alt="View on Strava" title="View on Strava" > </a> 
        {/if}
        {#if part.what != types[part.what].main}
            {types[part.what].name.toLowerCase()} 
        {/if}
        {#if !part.disposed_at}
        purchased <span class="param">{fmtDate(part.purchase)}</span>
         which
        {:else}
          you owned from <span class="param">{fmtDate(part.purchase)}</span>
          until <span class="param">{fmtDate(part.disposed_at)}</span>
          and 
        {/if}
        you used <span class=param>{fmtNumber(part.count)}</span> times 
        for <span class="param">{fmtSeconds(part.time)}</span> hours.
        <p> You covered <span class="param">{fmtNumber(parseFloat((part.distance / 1000).toFixed(1)))}</span> km 
        climbing <span class="param">{fmtNumber(part.climb)}</span> and descending <span class="param">{fmtNumber(part.descend)}</span> meters 
      </CardBody>
    <!-- </div> -->
  {/if}
</Card>
