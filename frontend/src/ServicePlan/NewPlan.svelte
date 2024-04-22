<script lang="ts">
  import {
    InputGroup,
    InputGroupText,
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
  } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import { Part } from "../lib/part";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import type { Type } from "../lib/types";
  import PlanForm from "./PlanForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import GearForm from "../Widgets/GearForm.svelte";

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
  <form on:submit|preventDefault={createPlan}>
    <ModalHeader {toggle}>New service plan for</ModalHeader>
    <ModalBody>
      {#if part.isGear()}
        <InputGroup class="col-md-12">
          <TypeForm with_body on:change={sethook} />
          <InputGroupText>of</InputGroupText>
          <GearForm bind:gear={plan.part} />
        </InputGroup>
      {:else}
        New service plan for {part.name}
      {/if}
      <PlanForm bind:plan />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Create"} />
    </ModalFooter>
  </form>
</Modal>
