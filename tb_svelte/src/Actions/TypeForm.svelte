<script lang="ts">
  import {
    Form, InputGroup, InputGroupAddon, InputGroupText, Input, FormGroup, Label,
  } from 'sveltestrap'
  import type {Type, Part, Attachment} from '../types'
  import {types, category, filterValues} from '../store'

  export let type: Type = undefined;
  export let gear: Part;
  export let attach: Attachment;

  let typeList = filterValues($types, (a: Type) => a.main == $category || 1 && a.id != a.main).sort((a: Type,b: Type) => a.order - b.order) as Type[];

  function updatehook() {
    attach.hook = (type.hooks.length == 1) ? type.hooks[0] : undefined
    attach.what = type.id
  }

</script>


<Form>
  <InputGroup class="col-md-12">
    <InputGroupAddon addonType="prepend">
      <InputGroupText>New</InputGroupText>
    </InputGroupAddon>
    <!-- svelte-ignore a11y-no-onchange -->
    <select  class="custom-select" required bind:value={type} on:change={updatehook}>
      <option hidden value> -- select one -- </option>
      {#each typeList as type}
        <option value={type}>{type.name}</option>
      {/each}
    </select> 
    {#if type && type.hooks.length > 1}
      <InputGroupAddon addonType="append">
        <InputGroupText>for </InputGroupText>
      </InputGroupAddon>
      <!-- svelte-ignore a11y-autofocus -->
      <select name="hook" class="form-control" required bind:value={attach.hook}>
        <option hidden value={undefined}> -- select one -- </option>
        {#each type.hooks as h}
          <option value={h}>{$types[h].name}</option>
        {/each}
      </select>
      <InputGroupText>of {gear.name}</InputGroupText>
    {:else}
      <InputGroupText>for {gear.name}</InputGroupText>
    {/if}
  </InputGroup>
</Form>