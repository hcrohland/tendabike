<script lang="ts">
  import {
    Input, InputGroup, FormGroup, Label, Col
  } from '@sveltestrap/sveltestrap'
  import DateTime from '../Widgets/DateTime.svelte';
  import {Type, Part} from '../lib/types'
  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher()

  export let type: Type|undefined;
  export let part: Part;
  export let maxdate: Date|undefined = undefined;
  export let mindate: Date|undefined  = undefined;
  let {name, vendor, model} = part

  $: if (type && name.length > 0 && vendor.length > 0 && model.length > 0) {
      part = new Part({...part, name, vendor, model})
      part.last_used = part.purchase
      dispatch ("change", part)  
    }
</script>

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
  <Col>
    <Label for="inputDate" class="text-end"> New {type && type.name || ''} day was </Label>
  </Col>
  <Col xs=auto>
    <InputGroup>
    <DateTime id="inputDate" bind:date={part.purchase} {maxdate} {mindate} required/>
    </InputGroup>
  </Col>
</FormGroup>