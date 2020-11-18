<script lang="ts">
  import {
    Form, Input, FormGroup, Label,
  } from 'sveltestrap'
  import DateTime from './DateTime.svelte';
  import type {Type, Part} from '../types'

  export let part: Part;
  export let type: Type;
  export let disabled = true;

  $: disabled = !(part && part.name.length > 0 && part.vendor.length > 0 && part.model.length > 0)
  
</script>

<Form>
  <FormGroup row>
    <FormGroup class="col-md-12">
      <Label for="inputName">You call it</Label>
      <!-- svelte-ignore a11y-autofocus -->
      <Input type="text" class="form-control" id="inputName" bind:value={part.name} autofocus required placeholder="Name" />
    </FormGroup>
  </FormGroup>
  <FormGroup row>
    <FormGroup class="col-md-6">
      <Label for="inputBrand">and it is a</Label>
      <Input type="text" class="form-control" id="inputBrand" bind:value={part.vendor} placeholder="Brand"/>
    </FormGroup>
    <FormGroup class=" col-md-6">
      <Label for="inputModel"> &nbsp. </Label>
      <Input type="text" class="form-control" id="inputModel" bind:value={part.model} placeholder="Model"/>
    </FormGroup>
  </FormGroup>
  <FormGroup row>
    <FormGroup class="col-md-6">
      <Label for="inputDate">New {type && type.name} day was at</Label>
      <DateTime id="inputDate" class="Input-group-text" bind:date={part.purchase} required/>
    </FormGroup>
  </FormGroup>
</Form>