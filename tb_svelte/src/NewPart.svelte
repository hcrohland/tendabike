<script lang="ts">
  import {
    Button,
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
    Spinner,
  } from 'sveltestrap';
  import DateTime from './DateTime.svelte';
  import {myfetch, initData, parts, user} from './store';
  import type {Type, Part} from './types'

  const toggle = () => isOpen = !isOpen

  let part: Part;
  let type: Type;
  let isOpen = false;
  let promise;

  async function savePart () {
    disabled = true;
    try {
      await myfetch('/part/', 'POST', part)
        .then(data => parts.updateMap([data]))
    } catch (e) {
      alert (e)
      initData()
    }
    isOpen = false;
  }

  export const popup = (t: Type) => {
    type = t;
    part = {
      owner: $user.id,
      what: type.id,
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: "",
      vendor: "",
      model: "",
      purchase: new Date(),
      last_used: new Date()
    };
    isOpen = true;
  }

  $: disabled = !(part && part.name.length > 0 && part.vendor.length > 0 && part.model.length > 0)
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> New {type.name} </ModalHeader>
  <ModalBody>
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
  </ModalBody>
  <ModalFooter>
    <Button color="secondary" on:click={toggle}>Cancel</Button>
    <Button color="primary" {disabled} on:click={() => (promise = savePart())}>
      {#await promise}
        <Spinner />
      {:then} 
        Attach
      {:catch error}
        {error}
      {/await}
    </Button>
  </ModalFooter>
</Modal>