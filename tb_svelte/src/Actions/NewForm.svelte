<script lang="ts">
  import DateTime from './DateTime.svelte';
  import type {Type, Part} from '../types'

  export let part: Part;
  export let type: Type;
  export let disabled = true;

  $: disabled = !(part && part.name.length > 0 && part.vendor.length > 0 && part.model.length > 0)
  
</script>


<form>
  <div class="form-row">
    <div class="form-group col-md-12">
      <label for="inputName">You call it</label>
      <!-- svelte-ignore a11y-autofocus -->
      <input type="text" class="form-control" id="inputName" bind:value={part.name} autofocus required placeholder="Name">
    </div>
  </div>
  <div class="form-row">
    
    <div class="form-group col-md-6">
      <label for="inputBrand">and it is a</label>
      <input type="text" class="form-control" id="inputBrand" bind:value={part.vendor} placeholder="Brand">
    </div>
    <div class="form-group col-md-6">
      <label class="d-none d-md-block" for="inputModel"> &nbsp </label>
      <input type="text" class="form-control" id="inputModel" bind:value={part.model} placeholder="Model">
    </div>
  </div>
  <div class="form-row">
    <div class="form-group col-md-6">
      <label for="inputDate">New {type.name} day was at</label>
      <DateTime id="inputDate" class="input-group-text" bind:date={part.purchase} required/>
    </div>
  </div>
</form>