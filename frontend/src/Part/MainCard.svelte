<GearCard {part} {isOpen} display>
  <div class="float-right">
    <a href={"/strava/bikes/" + part.id} alt="View on Strava" target="_blank"><img src="strava_grey.png" alt="View on Strava" title="View on Strava" > </a> 
    <a href="#/part/{part.id}" class="badge badge-secondary" title={"View "+ $category.name.toLowerCase() + " details"}>
      &Longrightarrow;
    </a>
  </div>
</GearCard>

<script lang="ts">
  import GearCard from './GearCard.svelte';
  import type { Part } from '../types';
  import {category} from '../store'

  export let part: Part
  export let isOpen: boolean

</script>