<script lang="ts">
  import { Modal, ModalBody, ModalHeader } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { ServicePlan } from "../lib/serviceplan";
  import { parts, Part } from "../lib/part";
  import { types } from "../lib/store";
  import PlanForm from "./PlanForm.svelte";

  let part: Part;
  let plan: ServicePlan;
  let isOpen = false;
  let update = false;

  async function postPlan() {
    await plan.update();
    isOpen = false;
  }

  export const updatePlan = (p: ServicePlan) => {
    part = $parts[p.part];
    plan = new ServicePlan(p);
    isOpen = true;
    update = true;
  };

  const toggle = () => {
    isOpen = false;
  };

  $: disabled = !(plan && plan.valid());
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Update service plan for
    {#if part.isGear() && plan.hook != null}
      {types[plan.what].human_name(plan.hook)} of {part.name}
    {:else}
      {part.name}
    {/if}
  </ModalHeader>
  <ModalBody>
    <PlanForm bind:plan />
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={postPlan} button={"Update"} />
</Modal>
