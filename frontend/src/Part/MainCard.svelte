<GearCard {part} {display}>
  <div class="float-end">
    <a href="#/part/{part.id}" class="badge badge-secondary text-decoration-none" title={"View "+ $category.name.toLowerCase() + " details"}>
      &Longrightarrow;
    </a>
  </div>
</GearCard>

<script lang="ts">
  import GearCard from './GearCard.svelte';
  import type { Part } from '../types';
  import {category} from '../store'

  export let part: Part
  export let display: boolean

</script>