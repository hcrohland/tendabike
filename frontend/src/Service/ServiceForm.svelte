<script lang="ts">
  import {
    Input,
    InputGroup,
    FormGroup,
    Label,
    Col,
  } from "@sveltestrap/sveltestrap";
  import DateTime from "../Widgets/DateTime.svelte";
  import { Service } from "./service";
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  export let service: Service;
  export let noname = false;
  export let maxdate: Date | undefined = undefined;
  export let mindate: Date | undefined = undefined;
  let { name, notes, time } = service;

  $: if (name.length > 0 && name.length > 0) {
    service = new Service({ ...service, name, notes, time });
    dispatch("change", service);
  }
</script>

<FormGroup row>
  <FormGroup class="col-md-12">
    <!-- svelte-ignore a11y-autofocus -->
    <Input
      type="text"
      class="form-control"
      id="inputName"
      bind:value={name}
      disabled={noname}
      autofocus
      required
      placeholder="Name"
    />
  </FormGroup>
</FormGroup>
<FormGroup row>
  <FormGroup>
    <Label for="inputNotes">Notes</Label>
    <Input
      type="textarea"
      class="form-control"
      id="inputNotes"
      bind:value={notes}
      placeholder="Notes"
    />
  </FormGroup>
</FormGroup>
<FormGroup row>
  <Col>
    <Label for="inputDate" class="text-end">at</Label>
  </Col>
  <Col xs="auto">
    <InputGroup>
      <DateTime id="inputDate" bind:date={time} {maxdate} {mindate} required />
    </InputGroup>
  </Col>
</FormGroup>
