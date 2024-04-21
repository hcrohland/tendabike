<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
  } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import { parts, Part } from "../lib/part";
  import { category, types } from "../lib/types";
  import PlanForm from "./PlanForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { attachments } from "../lib/attachment";

  let part: Part | null;
  let plan: ServicePlan;
  let isOpen = false;

  async function postPlan() {
    await plan.update();
    isOpen = false;
  }

  export const updatePlan = (p: ServicePlan) => {
    part = p.getpart($parts, $attachments);
    plan = new ServicePlan(p);
    isOpen = true;
  };

  const toggle = () => {
    isOpen = false;
  };

  $: disabled = !(plan && plan.valid());
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Update service plan for
    {#if part == null}
      {types[plan.what].human_name(plan.hook)} of any {$category.name.toLocaleLowerCase()}
    {:else if part.isGear() && plan.hook != null}
      {types[plan.what].human_name(plan.hook)} of {part.name}
    {:else}
      {part.name}
    {/if}
  </ModalHeader>
  <form on:submit|preventDefault={postPlan}>
    <ModalBody>
      <PlanForm bind:plan />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Update"} />
    </ModalFooter>
  </form>
</Modal>
