<script lang="ts">
  import { Modal, DropdownItem, Button } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";

  interface Props {
    plan: ServicePlan;
  }
  let { plan } = $props();

  let open = $state(false);

  async function onaction() {
    await plan.delete();
    open = false;
  }

  export const deletePlan = (p: ServicePlan) => {
    plan = p;
    open = true;
  };
</script>

<DropdownItem onclick={() => (open = true)}>Delete ServicePlan</DropdownItem>
<Modal form {open} {onaction}>
  {#snippet header()}
    Do you really want to delete ServicePlan <br />
    "{plan.name}"?
  {/snippet}
  <Button type="submit" value="confirm">Delete</Button>
  <Button type="submit">Cancel</Button>
</Modal>
