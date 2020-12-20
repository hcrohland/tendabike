<script lang="ts">
  import {Button, ButtonGroup} from 'sveltestrap'
  import {types, parts, category, activities, filterValues, act_types, myfetch, updateSummary, handleError} from '../store'
  import Subparts from './Subparts.svelte'
  import type {Part, Activity} from '../types'
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

  function unattached(acts: Activity[], g: Part){
    let myActTypes = filterValues($act_types, (t) => t.gear_type == g.what); 
    return Object.values($activities).some((a) => a.gear == null && myActTypes.some((t) => t.gear_type == a.what));
  }
  
  function defaultGear(g: Part) {
    myfetch('/activ/defaultgear', 'POST', g.id)
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
      {#if unattached($activities, gear)}
        <Button on:click={() => defaultGear(gear)}>  Assign this {$category.name.toLowerCase()} to activities without a {$category.name.toLowerCase()} </Button>
      {/if}
      {/if}
    </ButtonGroup>
</GearCard>
<Subparts {hook} {gear} />

<InstallPart bind:installPart />
<ChangePart bind:changePart />
<RecoverPart bind:recoverPart />