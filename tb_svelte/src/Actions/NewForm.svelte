<script lang="ts">
  import {
    Form, Input, FormGroup, Label,
  } from 'sveltestrap'
  import DateTime from './DateTime.svelte';
  import type {Type, Part} from '../types'
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher()

  export let type: Type;
  export let part: Part;
  let {name, vendor, model, purchase} = part

  $: if (type && name.length > 0 && vendor.length > 0 && model.length > 0) {
      part = {...part, name, vendor, model, purchase}
      part.last_used = purchase
      dispatch ("change", part)  
    }
</script>

<Form>
  <FormGroup row>
    <FormGroup class="col-md-12">
      <Label for="inputName">You call it</Label>
      <!-- svelte-ignore a11y-autofocus -->
      <Input type="text" class="form-control" id="inputName" bind:value={name} autofocus required placeholder="Name" />
    </FormGroup>
  </FormGroup>
  <FormGroup row>
    <FormGroup class="col-md-6">
      <Label for="inputBrand">and it is a</Label>
      <Input type="text" class="form-control" id="inputBrand" bind:value={vendor} placeholder="Brand"/>
    </FormGroup>
    <FormGroup class=" col-md-6">
      <Label for="inputModel"> &nbsp. </Label>
      <Input type="text" class="form-control" id="inputModel" bind:value={model} placeholder="Model"/>
    </FormGroup>
  </FormGroup>
  <FormGroup row>
    <FormGroup class="col-md-6">
      <Label for="inputDate">New {type && type.name || ''} day was </Label>
      <DateTime id="inputDate" class="Input-group-text" bind:date={purchase} required/>
    </FormGroup>
  </FormGroup>
</Form>