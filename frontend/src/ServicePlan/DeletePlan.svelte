<script lang="ts">
  import { Modal, Button } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";

  let plan = $state(new ServicePlan({}));

  let open = $state(false);

  async function onaction() {
    await plan.delete();
    open = false;
  }

  export const start = (p: ServicePlan) => {
    plan = p;
    open = true;
  };
</script>

<Modal form {open} {onaction}>
  {#snippet header()}
    Do you really want to delete ServicePlan <br />
    "{plan.name}"?
  {/snippet}
  <Button type="submit" value="confirm">Delete</Button>
  <Button type="submit">Cancel</Button>
</Modal>
