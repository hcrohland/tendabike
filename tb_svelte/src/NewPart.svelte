<script>
  import Modal from './Modal.svelte';
  import DateTime from './DateTime.svelte';
  import {myfetch, types, parts} from './store.js';
  
  export let cat;
  export let title = 'New ' + cat.name;

  let name;
  let vendor;
  let model;
  let purchase;

  let showModal = false;

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

<span type="button" class="badge badge-secondary" on:click="{() => showModal = true}">
  {title}
</span>

{#if showModal}
  <Modal save="Create" on:close="{() => showModal = false}">
    <span slot="header"> New {cat.name} </span>
    <form>
      <div class="form-row">
        <div class="form-group col-md-12">
          <label for="inputName">You call it</label>
          <input type="text" class="form-control" id="inputName" bind:value={name} required placeholder="Name">
        </div>
      </div>
      <div class="form-row">

      <div class="form-group col-md-6">
        <label for="inputBrand">and it is a</label>
        <input type="text" class="form-control" id="inputBrand" bind:value={vendor} placeholder="Brand">
      </div>
      <div class="form-group col-md-6">
        <label class="d-none d-md-block" for="inputModel"> &nbsp </label>
        <input type="text" class="form-control" id="inputModel" bind:value={model} placeholder="Model">
      </div>
      </div>
      <div class="form-row">
        <div class="form-group col-md-6">
          <label for="inputDate">New {cat.name} day was at</label>
          <DateTime id="inputDate" bind:date={purchase} required/>
        </div>
      </div>
    </form>
    <span slot="footer">
      <button type="submit" class="btn btn-primary float-right" on:click={savePart}>Create {cat.name}</button>
    </span>
  </Modal>
{/if}