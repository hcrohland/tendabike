<script module lang="ts">
  import { ServicePlan } from "../lib/serviceplan";

  const modal = $state<{
    open: boolean;
    plan: ServicePlan;
  }>({
    open: false,
    plan: new ServicePlan({}),
  });

  export const start = (p: ServicePlan) => {
    modal.plan = p;
    modal.open = true;
  };
</script>

<script lang="ts">
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.plan.delete();
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.deleteplan_header({ name: modal.plan.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_delete()} />
  {/snippet}
</Modal>
