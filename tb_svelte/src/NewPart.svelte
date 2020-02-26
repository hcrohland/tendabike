<script>
  import Modal from './Modal.svelte';
  import DateTime from './DateTime.svelte';
  import {myfetch, types, initData, parts, user} from './store.js';
  import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher();

  export let cat;
  export let title = 'New ' + cat.name;

  let part;
  let enabled = false;
  let showModal = false;

  async function savePart () {
    disabled = true;
    try {
      await myfetch('/part/', 'POST', part)
        .then(data => parts.updateMap([data]))
        .then(dispatch('created'))
    } catch (e) {
      alert (e)
      initData()
    }
    showModal = false;
}

  function popup(){
    part = {
      owner: $user.id, 
      what: cat.id, 
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: "", 
      vendor: "", 
      model: "", 
      purchase: new Date()
    };
    showModal = true;
  }

  $: disabled = !(part && part.name.length > 0 && part.vendor.length > 0 && part.model.length > 0)
</script>
<span type="button" class="badge badge-secondary" on:click="{popup}">
  {title}
</span>

{#if showModal}
  <Modal save="Create" on:close="{() => showModal = false}">
    <span slot="header"> New {cat.name} </span>
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
          <label for="inputDate">New {cat.name} day was at</label>
          <DateTime id="inputDate" class="input-group-text" bind:date={part.purchase} required/>
        </div>
      </div>
    </form>
    <span slot="footer">
      <button type="submit" {disabled} class="btn btn-primary float-right" on:click={savePart}>Create {cat.name}</button>
    </span>
  </Modal>
{/if}