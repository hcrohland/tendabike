<script lang="ts">
  import {filterValues, types, parts, attachments, category, by} from '../store'
  import PartHist from './PartHist.svelte'
  import GearCard from './GearCard.svelte'

  export let params;
  
  let part 
  
  $: { 
    part = $parts[params.id]
    category.set($types[$types[part.what].main])
  }

  $: atts = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))
  
</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>

<div class="scroll-x">
  <GearCard {part} display/>
  <PartHist id={part.id} />
</div>