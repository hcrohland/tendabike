<script lang="ts">
  import {Button, ButtonGroup} from 'sveltestrap'
  import {types, parts, category, activities, myfetch, updateSummary, handleError} from '../store'
  import Subparts from './Subparts.svelte'
  import InstallPart from '../Actions/InstallPart.svelte'
  import ChangePart from '../Actions/ChangePart.svelte'
  import RecoverPart from '../Actions/RecoverPart.svelte';
  import GearCard from '../Part/GearCard.svelte'
  import PartHist from '../Part/PartHist.svelte'
 
  export let params;
  
  let installPart, changePart, recoverPart;

  $: category.set(hook)

  $: gear = $parts[params.id]; 
  $: hook = types[gear.what];
  $: unassigned = Object
      .values($activities)
      .some((a) => a.gear == null && hook.acts.some((t) => t.gear_type == a.what))
  
  function defaultGear(id: number) {
    myfetch('/activ/defaultgear', 'POST', id)
      .then(updateSummary)
      .catch(handleError)
  }
</script>

<GearCard part={gear} display>
  <ButtonGroup class="float-right">
      {#if gear.disposed_at}
      <Button on:click={() => recoverPart(gear)}> Recover gear </Button>
      {:else}
      <Button on:click={() => installPart(gear)}>  Install new part </Button>
      <Button on:click={() => changePart(gear)}>  Change details </Button>
      {#if unassigned}
        <Button on:click={() => defaultGear(gear.id)}>  Assign this {hook.name.toLowerCase()} to activities without a {hook.name.toLowerCase()} </Button>
      {/if}
      {/if}
    </ButtonGroup>
</GearCard>
<PartHist id={gear.id} />

<Subparts {hook} {gear} />

<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />