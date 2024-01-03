<script lang="ts">
  import {ButtonGroup, Button} from '@sveltestrap/sveltestrap'
  import {types, parts} from '../store'
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
  
</script>

<GearCard {part} display>
  <ButtonGroup class="float-end">
      {#if part.disposed_at}
      <Button on:click={() => recoverPart(part)}> Recover gear </Button>
      {:else}
      {#if part.what == hook.main}
         <Button color=light on:click={() => installPart(part)}>  Install new part </Button>
      {:else}
         <Button color=light on:click={() => attachPart(part)}>  Attach part </Button>
      {/if}
      <Button color=light on:click={() => changePart(part)}>  Change details </Button>
      {/if}
  </ButtonGroup>
</GearCard>

<PartHist id={part.id} />
<Subparts gear={part} {hook} />

<AttachPart bind:attachPart />
<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />