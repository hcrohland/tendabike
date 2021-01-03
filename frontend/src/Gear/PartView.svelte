<script lang="ts">
import { Button, ButtonGroup} from 'sveltestrap';
import {types, parts, category} from '../store'
import type { Part, Type} from '../types'
import PartHist from './PartHist.svelte'
import GearCard from './GearCard.svelte'
import ChangePart from '../Actions/ChangePart.svelte'
import AttachPart from '../Actions/AttachPart.svelte'
import RecoverPart from '../Actions/RecoverPart.svelte'

export let params;

let changePart, attachPart, recoverPart
let part: Part;
let type: Type;
  
$: { 
  part = $parts[params.id]
  type = types[part.what]
  category.set(types[type.main])
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
    <ButtonGroup class="float-right">
        {#if part.disposed_at}
        <Button on:click={() => recoverPart(part)}> Recover {type.name} </Button>
        {:else}
        <Button on:click={() => attachPart(part)}>  Attach part </Button>
        <Button on:click={() => changePart(part)}>  Change details </Button>
        {/if}
      </ButtonGroup>
  </GearCard>
  <PartHist id={part.id} />
</div>
<ChangePart bind:changePart />
<AttachPart bind:attachPart />
<RecoverPart bind:recoverPart />
