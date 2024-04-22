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

  let name: string | null;
  let plan: ServicePlan;
  let isOpen = false;

  async function createPlan() {
    await plan.create();
    isOpen = false;
  }

  export const newPlan = (p?: Part) => {
    if (p && !p.isGear()) {
      name = p.name;
      plan = new ServicePlan({ part: p.id, what: p.what, hook: null });
    } else {
      name = null;
      plan = new ServicePlan({ part: p?.id });
    }
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
      New service plan for
      {#if name}
        {name}
      {/if}
    </ModalHeader>
    <ModalBody>
      {#if !name}
        <InputGroup class="col-md-12">
          <TypeForm with_body on:change={sethook} />
          <InputGroupText>of</InputGroupText>
          <GearForm bind:gear={plan.part} />
        </InputGroup>
      {/if}
      <PlanForm bind:plan />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Create"} />
    </ModalFooter>
  </form>
</Modal>
