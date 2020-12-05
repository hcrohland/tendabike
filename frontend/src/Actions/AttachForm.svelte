<script lang="ts">
  import {
    Form, InputGroup, InputGroupAddon, InputGroupText
  } from 'sveltestrap'
  import DateTime from './DateTime.svelte';
  import {attachments, filterValues, types, parts, by} from '../store';
  import type {Attachment, Type, Part} from '../types';  

  
  function lastDetach(part) {
    let last = filterValues<Attachment>($attachments, (a) => a.part_id == part.id).sort(by("attached"))[0]
      
    if (last) {
      return new Date(last.detached ? last.detached : last.attached)
    } else {
      return new Date(part.purchase)
    }
  }
    
  export let attach: Attachment;
  export let part: Part;
  export let disabled = true;

  let type: Type = $types[part.what]; 
  let options: Part[] = filterValues($parts, (p) => type.main == p.what);
  
  attach = {
    part_id: part.id,
    attached: lastDetach(part),
    gear: undefined,
    hook: (type.hooks.length == 1) ? type.hooks[0] : undefined,
    detached: null,
  } 
  
  $: disabled = attach && !($types[attach.hook] && $parts[attach.gear])
</script>

<Form>
  <div class="form-inline">
    <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
      <InputGroupAddon addonType="prepend">
        <InputGroupText>to</InputGroupText>
      </InputGroupAddon>
      {#if type.hooks.length > 1}
        <!-- svelte-ignore a11y-autofocus -->
        <select name="hook" class="form-control" required bind:value={attach.hook}>
          <option hidden value={undefined}> -- select one -- </option>
          {#each type.hooks as h}
            <option value={h}>{$types[h].name}</option>
          {/each}
        </select>
        <InputGroupAddon addonType="prepend">
          <InputGroupText>of</InputGroupText>
        </InputGroupAddon>
      {/if}
      <select name="gear" class="form-control" required bind:value={attach.gear}>
        <option hidden value> -- select one -- </option>
        {#each options as gear}
          <option value={gear.id}>{gear.name}</option>
        {/each}
      </select> 
    </InputGroup>  
    <InputGroup class="mb-0 mr-sm-2 mb-sm-2">
      <InputGroupAddon addonType="prepend">
        <InputGroupText>at</InputGroupText>
      </InputGroupAddon>
      <DateTime bind:date={attach.attached}/> 
    </InputGroup>
  </div>
</Form> 