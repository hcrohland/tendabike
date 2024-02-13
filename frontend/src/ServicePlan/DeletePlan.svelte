<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { ServicePlan } from "./serviceplan";

  let plan: ServicePlan;
  let isOpen = false;
  const toggle = () => (isOpen = false);

  async function action() {
    await plan.delete();
    isOpen = false;
  }

  export const deletePlan = (p: ServicePlan) => {
    plan = p;
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Do you really want to delete ServicePlan <br />
    "{plan.name}"?
  </ModalHeader>
  <ModalFooter {toggle} {action} button={"Delete"} />
</Modal>
