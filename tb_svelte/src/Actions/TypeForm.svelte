<script lang="ts">
  import type {Type, Part} from '../types'
  import {types, category, filterValues} from '../store'

  export let type: Type = undefined;
  export let gear: Part;
  export let hook;

  let typeList = filterValues($types, (a: Type) => a.main == 1 && a.id != a.main).sort((a: Type,b: Type) => a.order - b.order) as Type[];

  function updatehook() {
    hook = (type.hooks.length == 1) ? type.hooks[0] : undefined
  }

</script>


<form>
    <div class="input-group col-md-12">
      <div class="input-group-prepend">
        <span class="input-group-text">New</span>
      </div>
      <!-- svelte-ignore a11y-no-onchange -->
      <select  class="custom-select" required bind:value={type} on:change={updatehook}>
        <option hidden value> -- select one -- </option>
        {#each typeList as type}
          <option value={type}>{type.name}</option>
        {/each}
      </select> 
      {#if type && type.hooks.length > 1}
        <div class="input-group-append">
          <span class="input-group-text">for </span>
        </div>
        <!-- svelte-ignore a11y-autofocus -->
        <select name="hook" class="form-control" required bind:value={hook}>
          <option hidden value={undefined}> -- select one -- </option>
          {#each type.hooks as h}
            <option value={h}>{$types[h].name}</option>
          {/each}
        </select>
          <span class="input-group-text">of {gear.name}</span>
        {:else}
          <span class="input-group-text">for {gear.name}</span>

        {/if}
    </div>
</form>