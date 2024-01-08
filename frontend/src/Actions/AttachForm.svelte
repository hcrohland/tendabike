<script lang="ts">
  import {
    Form, InputGroup, InputGroupText
  } from '@sveltestrap/sveltestrap'
  import DateTime from '../Widgets/DateTime.svelte';
  import {attachments, filterValues, types, parts, by} from '../lib/store';
  import {type AttEvent, maxDate, Part} from '../lib/types';  

  
  function lastDetach(part) {
    let last = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))[0]
      
    if (last) {
      return last.detached < maxDate ? last.detached : last.attached
    } else {
      return part.purchase
    }
  }
    
  export let attach: AttEvent;
  export let part: Part;
  export let disabled = true;

  let type = types[part.what]; 
  let options = filterValues($parts, (p) => (type.main == p.what && ! p.disposed_at));
  
  attach = {
    part_id: part.id,
    time: lastDetach(part),
    gear: undefined,
    hook: (type.hooks.length == 1) ? type.hooks[0] : undefined,
  } 
  
  $: disabled = attach && !(types[attach.hook] && $parts[attach.gear])
</script>

<Form>
  <div class="form-inline">
    <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
      <InputGroupText>to</InputGroupText>
      {#if type.hooks.length > 1}
        <!-- svelte-ignore a11y-autofocus -->
        <select name="hook" class="form-control" required bind:value={attach.hook}>
          <option hidden value={undefined}> -- select one -- </option>
          {#each type.hooks as h}
            <option value={h}>{types[h].name}</option>
          {/each}
        </select>
        <InputGroupText>of</InputGroupText>
      {/if}
      <select name="gear" class="form-control" required bind:value={attach.gear}>
        <option hidden value> -- select one -- </option>
        {#each options as gear}
          <option value={gear.id}>{gear.name}</option>
        {/each}
      </select> 
    </InputGroup>  
    <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
      <InputGroupText>at</InputGroupText>
      <DateTime bind:date={attach.time}/> 
    </InputGroup>
  </div>
</Form> 