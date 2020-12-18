<script lang="ts">
  import {Button, ButtonGroup} from 'sveltestrap'
  import {types, parts, category} from '../store'
  import Subparts from './Subparts.svelte'
  import type {Part} from '../types'
  import InstallPart from '../Actions/InstallPart.svelte'
  import ChangePart from '../Actions/ChangePart.svelte'
  import RecoverPart from '../Actions/RecoverPart.svelte';
  import GearCard from '../Part/GearCard.svelte'
 
  export let params;
  
  let hook, gear: Part;
  let installPart, changePart, recoverPart;

  $: {
    gear = $parts[params.id]; 
    hook = $types[gear.what];
    category.set(hook)
  }
  
</script>

<GearCard part={gear} display>
  <ButtonGroup class="float-right">
      {#if gear.disposed_at}
      <Button on:click={() => recoverPart(gear)}> Recover gear </Button>
      {:else}
      <Button on:click={() => installPart(gear)}>  Install new part </Button>
      <Button on:click={() => changePart(gear)}>  Change details </Button>
      {/if}
    </ButtonGroup>
</GearCard>
<Subparts {hook} {gear} />

<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />