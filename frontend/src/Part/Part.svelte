<script lang="ts">
  import {ButtonGroup, Button} from 'sveltestrap'
  import {types, parts, activities, myfetch, updateSummary, handleError} from '../store'
  import InstallPart  from '../Actions/InstallPart.svelte'
  import ChangePart   from '../Actions/ChangePart.svelte'
  import RecoverPart  from '../Actions/RecoverPart.svelte';
  import AttachPart   from '../Actions/AttachPart.svelte';
  import Subparts from './Subparts.svelte'
  import GearCard from './GearCard.svelte'
  import PartHist from './PartHist.svelte'
 
  export let params;
  
  let installPart, changePart, recoverPart, attachPart;

  $: part = $parts[params.id]; 
  $: hook = types[part.what];
  $: unassigned = Object
      .values($activities)
      .some((a) => a.gear == null && hook.acts.some((t) => t.gear_type == a.what))
  
  function defaultGear(id: number) {
    myfetch('/activ/defaultgear', 'POST', id)
      .then(updateSummary)
      .catch(handleError)
  }
</script>

<GearCard {part} display>
  <ButtonGroup class="float-right">
      {#if part.disposed_at}
      <Button on:click={() => recoverPart(part)}> Recover gear </Button>
      {:else}
      {#if part.what == hook.main}
         <Button on:click={() => installPart(part)}>  Install new part </Button>
      {:else}
         <Button on:click={() => attachPart(part)}>  Attach part </Button>
      {/if}
      <Button on:click={() => changePart(part)}>  Change details </Button>
      {#if unassigned}
        <Button on:click={() => defaultGear(part.id)}>  Assign this {hook.name.toLowerCase()} to activities without a {hook.name.toLowerCase()} </Button>
      {/if}
      {/if}
  </ButtonGroup>
</GearCard>

<PartHist id={part.id} />
<Subparts gear={part} {hook} />

<AttachPart bind:attachPart />
<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />