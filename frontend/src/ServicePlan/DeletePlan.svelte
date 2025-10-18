<script lang="ts">
  import { Modal } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";
  import Buttons from "../Widgets/Buttons.svelte";

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

<Modal form bind:open {onaction}>
  {#snippet header()}
    Do you really want to delete ServicePlan <br />
    "{plan.name}"?
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label="Delete" />
  {/snippet}
</Modal>
