<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    Form, InputGroup, InputGroupText, Input, FormGroup, Label,
  } from '@sveltestrap/sveltestrap'
  import type {Type, Part, Attachment} from '../types'
  import {types, category, filterValues} from '../store'

  export let gear: Part;

  let type: Type;
  let hook: number;
  const dispatch = createEventDispatcher();


  let typeList = filterValues(types, (t) => t.main == $category.id && t.id != t.main)
          .sort((a,b) => a.order - b.order);

  function updatehook() {
    hook = (type.hooks.length == 1) ? type.hooks[0] : undefined
  }

  $: if (hook && type) dispatch("change", {type, hook})
</script>

<Form>
  <InputGroup class="col-md-12">
    <InputGroupText>New</InputGroupText>
    <!-- svelte-ignore a11y-no-onchange -->
    <select  class="custom-select" required bind:value={type} on:change={updatehook}>
      <option hidden value> -- select one -- </option>
      {#each typeList as type}
        <option value={type}>{type.name}</option>
      {/each}
    </select> 
    {#if type && type.hooks.length > 1}
      <InputGroupText>for </InputGroupText>
      <!-- svelte-ignore a11y-autofocus -->
      <select name="hook" class="form-control" required bind:value={hook}>
        <option hidden value={undefined}> -- select one -- </option>
        {#each type.hooks as h}
          <option value={h}>{types[h].name}</option>
        {/each}
      </select>
      <InputGroupText>of {gear.name}</InputGroupText>
    {:else}
      <InputGroupText>for {gear.name}</InputGroupText>
    {/if}
  </InputGroup>
</Form>