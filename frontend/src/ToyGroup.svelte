<script lang="ts">
  import { Col, Row } from 'sveltestrap';
  import MainCard from './Gear/MainCard.svelte';
  import {filterValues, by, types, parts, category} from './store';
  import ShowAll from './Widgets/ShowHist.svelte';

  export let params = {category: 1};
  
  // Cannot use category directly since it 
  // is unset during destroy and the router gets confused
  let type, show_hist;
  if (params) {
    type = types[params.category];
  } else {
    type = types[1];
  }
  category.set(type);
  
  $: gears = filterValues($parts, (p) => p.what == type.id && ! p.disposed_at).sort(by("last_used"))
  $: bin = filterValues($parts, (p) => p.what == type.id && p.disposed_at != undefined).sort(by("last_used"))
</script>

{#if type }
  <Row border class="p-sm-2">
    {#each gears as part, i  (part.id)}
      <Col md=6 class="p-0 p-sm-2">
        <MainCard {part} isOpen={i<4} />
      </Col>
    {:else}
      You have no {type.name} to tend 😱
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
        <MainCard {part} isOpen={i<2} />
      </Col>
     {/each}
    </Row>
    {/if}
  {/if}
{:else}
     Error: Category not found!
{/if}