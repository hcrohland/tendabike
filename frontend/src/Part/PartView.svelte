<script lang="ts">
import {DropdownItem, Button} from 'sveltestrap';
import {filterValues, types, parts, attachments, category, by} from '../store'
import type {Attachment, Part, Type} from '../types'
import PartHist from './PartHist.svelte'
import GearCard from './GearCard.svelte'
import ChangePart from '../Actions/ChangePart.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import RecoverPart from '../Actions/RecoverPart.svelte'
import Menu from '../Menu.svelte'
import AttachForm from '../Actions/AttachForm.svelte';

export let params;

let changePart, attachPart, recoverPart
let part: Part;
let atts: Attachment[];
let type: Type;
let last: Attachment, start, stop;
  
$: { 
  part = $parts[params.id]
  type = $types[part.what]
  category.set($types[type.main])
}

$: {
  atts = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))
  start = atts[0] ? atts[0].attached : undefined
  last = atts[atts.length - 1]
}
  
</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>

<div class="scroll-x">
  <GearCard {part} display>
    <!-- <Menu> -->
      {#if part.disposed_at}
      <Button class="float-right" inline on:click={() => recoverPart(part)}> Recover {type.name} </Button>
      {:else}
      <Button class="float-right" inline on:click={() => changePart(part)}> 
        Change details 
      </Button>
      {/if}
    <!-- </Menu> -->
  </GearCard>
  <PartHist id={part.id} />
</div>
<ChangePart bind:changePart />
<AttachPart bind:attachPart />
<RecoverPart bind:recoverPart />
