<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    Form,
    FormGroup,
    Input,
  } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { ServicePlan } from "./serviceplan";
  import { Part } from "../Part/part";
  import PlanLimits from "./PlanLimits.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import type { Type } from "../lib/types";

  let part: Part;
  let plan: ServicePlan;
  let isOpen = false;

  async function createPlan() {
    await plan.create();
    isOpen = false;
  }

  export const newPlan = (p: Part) => {
    part = p;
    plan = new ServicePlan(
      part.isGear()
        ? { part: part.id }
        : { part: part.id, what: part.what, hook: null },
    );
    isOpen = true;
  };

  const toggle = () => {
    isOpen = false;
  };

  const sethook = (e: CustomEvent<{ type: Type; hook: number }>) => {
    plan.what = e.detail.type.id;
    plan.hook = e.detail.hook;
  };

  $: disabled = !(plan && plan.valid());
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    {#if part.isGear()}
      <TypeForm gear={part} with_body on:change={sethook}>
        New service plan for
      </TypeForm>
    {:else}
      New service plan for {part.name}
    {/if}
  </ModalHeader>
  <ModalBody>
    <Form>
      <FormGroup class="col-md-12">
        <!-- svelte-ignore a11y-autofocus -->
        <Input
          type="text"
          class="form-control"
          id="inputName"
          bind:value={plan.name}
          autofocus
          required
          placeholder="Name"
        />
      </FormGroup>
      <PlanLimits bind:select={plan} />
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={createPlan} button={"Create"} />
</Modal>
