<script lang="ts">
  import type {Type, Part} from '../types'
  import {types, category, filterValues} from '../store'

  export let type: Type = undefined;
  export let gear: Part;

  let typeList = filterValues($types, (a: Type) => a.main == 1 && a.id != a.main).sort((a: Type,b: Type) => a.order - b.order) as Type[];
</script>


<form>
    <div class="input-group col-md-12">
      <div class="input-group-prepend">
        <span class="input-group-text">New</span>
      </div>
      <select  class="custom-select" required bind:value={type}>
        <option hidden value> -- select one -- </option>
        {#each typeList as type}
        <option value={type}>{type.name}</option>
        {/each}
      </select> 
      <div class="input-group-append">
        <span class="input-group-text">for {gear.name}</span>
      </div>
    </div>
</form>