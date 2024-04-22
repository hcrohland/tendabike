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
    <ModalHeader {toggle}>
      {#if part.isGear()}
        <InputGroup class="col-md-12">
          <InputGroupText>New service plan for</InputGroupText>
          <TypeForm with_body on:change={sethook} />
          <InputGroupText>of {part.name}</InputGroupText>
        </InputGroup>
      {:else}
        New service plan for {part.name}
      {/if}
    </ModalHeader>
    <ModalBody>
      <PlanForm bind:plan />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Create"} />
    </ModalFooter>
  </form>
</Modal>
