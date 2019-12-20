<script>
  import Await from './Await.svelte';
  import Modal from './Modal.svelte';
  import {myfetch, filterValues, types, parts} from './store.js';
  
  export let cat;

  let name;
  let vendor;
  let model;
  let purchase;

  function savePart () {
    if (name) {
      let part = {id: 999, 
        what: cat.id, 
        count:0, climb:0, descend:0, distance:0, time: 0,
        name, vendor, model, purchase};
      parts.updateMap([part])
    }
  }

</script>

<Modal title='New {cat.name}' submit={savePart} save="Create">

<form>
  <div class="form-row">
    <div class="form-group col-md-12">
      <label for="inputName">You call your new {cat}</label>
      <input type="text" class="form-control" id="inputName" bind:value={name} required placeholder="Name">
    </div>
  </div>
  <div class="form-row">

  <div class="form-group col-md-6">
    <label for="inputBrand">and it is a</label>
    <input type="text" class="form-control" id="inputBrand" bind:value={vendor} placeholder="Brand">
  </div>
  <div class="form-group col-md-6">
    <label for="inputModel"> &nbsp </label>
    <input type="text" class="form-control" id="inputModel" bind:value={model} placeholder="Model">
  </div>
  </div>
  <div class="form-row">
    <div class="form-group col-md-6">
      <label for="inputDate">New {cat.name} day was at</label>
      <input type="date" class="form-control" id="inputDate" bind:value={purchase} required>
    </div>
  </div>
  <!-- <div class="form-group">
    <div class="form-check">
      <input class="form-check-input" type="checkbox" id="gridCheck">
      <label class="form-check-label" for="gridCheck">
        Check me out
      </label>
    </div>
  </div> -->
  <!-- <button type="submit" class="btn btn-primary">Create</button> -->
</form>
</Modal>