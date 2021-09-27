<GearCard {part} {display}>
  <div class="float-right">
    <a href="#/part/{part.id}" class="badge badge-secondary text-reset" title={"View "+ $category.name.toLowerCase() + " details"}>
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