<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

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

<Modal bind:open {onaction}>
  {#snippet header()}
    {m.deleteplan_header({ name: plan.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_delete()} />
  {/snippet}
</Modal>
