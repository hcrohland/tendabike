<script module lang="ts">
  import { Part } from "../lib/part";
  import { ServicePlan } from "../lib/serviceplan";

  const modal = $state<{
    open: boolean;
    plan: ServicePlan;
    part: Part;
    fraction: string;
  }>({
    open: false,
    plan: new ServicePlan({}),
    part: new Part({}),
    fraction: "",
  });

  export const snoozePlan = (p: ServicePlan, part: Part) => {
    modal.plan = p;
    modal.part = part;
    modal.fraction = "";
    modal.open = true;
  };
</script>

<script lang="ts">
  import { Input } from "flowbite-svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.plan.snooze(modal.part.id!, parseFloat(modal.fraction));
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.snoozeplan_header({ name: modal.part.name })}
  {/snippet}
  <Input
    type="number"
    min="0"
    step="any"
    required
    autofocus
    bind:value={modal.fraction}
    placeholder={m.snoozeplan_fraction()}
  />
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_snooze()} />
  {/snippet}
</Modal>
