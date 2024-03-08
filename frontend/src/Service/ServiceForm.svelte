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
  import Switch from "../Widgets/Switch.svelte";
  import { plans, plans_for_part } from "../ServicePlan/serviceplan";
  import { attachments } from "../lib/attachment";
  import { parts } from "../lib/part";
  const dispatch = createEventDispatcher();

  export let service: Service;
  export let noname = false;
  export let finish = false;
  export let maxdate: Date | undefined = undefined;
  export let mindate: Date | undefined = undefined;
  let { name, notes, time } = service;
  let done = false;
  let redone: Date;

  $: if (name.length > 0) {
    let s = new Service({ ...service, name, notes, time, redone });
    dispatch("change", s);
  }

  $: planlist = plans_for_part(
    $parts[service.part_id],
    time,
    $plans,
    $attachments,
  );
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
{#if planlist.length > 0}
  <FormGroup row>
    <Col xs="auto">Resolves:</Col>
    <Col xs="auto">
      {#each planlist as plan (plan.id)}
        <div class="form-check">
          <label class="form-check-label">
            <input
              class="form-check-input"
              name="plans"
              type="checkbox"
              value={plan.id}
              bind:group={service.plans}
            />
            {plan.name}
          </label>
        </div>
      {/each}
    </Col>
    <br />
  </FormGroup>
{/if}
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
{#if finish}
  <FormGroup row>
    <Col>
      <InputGroup>
        <Switch bind:checked={done}>
          {#if done}
            until
          {:else}
            finish?
          {/if}
        </Switch>
      </InputGroup>
    </Col>
    <Col xs="auto">
      {#if done}
        <InputGroup>
          <DateTime bind:date={redone} mindate={time} />
        </InputGroup>
      {/if}
    </Col>
  </FormGroup>
{/if}
