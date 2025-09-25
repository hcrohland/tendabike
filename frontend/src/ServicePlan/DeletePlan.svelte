<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "flowbite-svelte";
  import MyFooter from "../Widgets/MyFooter.svelte";
  import { ServicePlan } from "../lib/serviceplan";

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

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>
    Do you really want to delete ServicePlan <br />
    "{plan.name}"?
  </ModalHeader>
  <MyFooter {toggle} {action} label={"Delete"} />
</Modal>
