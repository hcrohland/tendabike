<script lang="ts">
  import { Col, Row } from '@sveltestrap/sveltestrap';
  import MainCard from './Part/MainCard.svelte';
  import {filterValues, by, types, parts, category, activities} from './lib/store';
  import ShowAll from './Widgets/ShowHist.svelte';
  import SetDefault from './Actions/SetDefault.svelte';

  export let params: {category: number};
  
  // Cannot use category directly since it 
  // is unset during destroy and the router gets confused
  let show_hist: boolean;

  $: type = params ? types[params.category] : types[1]; 
  $: category.set(type); 
  $: gears = filterValues($parts, (p) => p.what == type.id && ! p.disposed_at).sort(by("last_used"))
  $: bin = filterValues($parts, (p) => p.what == type.id && p.disposed_at != undefined).sort(by("last_used"))

</script>

{#if type }
  <SetDefault {type}></SetDefault>
  <Row class="p-sm-2">
    {#each gears as part, i  (part.id)}
      <Col md=6 class="p-0 p-sm-2">
        <MainCard {part} display={i<4 || show_hist} />
      </Col>
    {:else}
      {#if type.activities($activities).length == 0}
         We did not find any {type.name} activities on Strava (yet). 
      {:else}
         You have no {type.name} assigned to any activity on Strava. Please do so to get started.
      {/if}
    {/each}
  </Row>
  
  {#if bin.length > 0}
  <ShowAll bind:show_hist>
     Show disposed 
  </ShowAll>
  {#if show_hist}
    <Row>
      {#each bin as part, i  (part.id)}
      <Col md=6 class="p-0 p-sm-2">
        <MainCard {part} display />
      </Col>
     {/each}
    </Row>
    {/if}
  {/if}
{:else}
     Error: Category not found!
{/if}