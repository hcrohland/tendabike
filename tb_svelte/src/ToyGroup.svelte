<script lang="ts">
  import GearCard from './GearCard.svelte';
  import {filterValues, by, types, parts, category} from './store';
  import type { Part } from './types';

  export let params = {category: 1};
  
  // Cannot use category directly since it 
  // is unset during destroy and the router gets confused
  let type;
  if (params) {
    type = $types[params.category];
  } else {
    type = $types[1];
  }
  category.set(type);
  
  $: gears = () => filterValues($parts, (p) => p.what == type.id) as Part[]
</script>

{#if type }
  <div class="row border p-sm-2">
    {#each gears().sort(by("last_used")) as part, i  (part.id)}
      <div class="col-md-6 p-0 p-sm-2">
        <GearCard {part} isOpen={i<4}/>
      </div>
    {:else}
      You have no {type.name} to tend 😱
    {/each}
  </div>
  <!-- <div class="container">
      <div class="row p-sm-2 float-right">
        <NewPart {type}/>
      </div>
  </div> -->
{:else}
     Error: Category not found!
{/if}