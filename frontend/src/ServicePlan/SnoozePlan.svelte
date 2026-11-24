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
  import { Button, ButtonGroup, Input, InputAddon } from "flowbite-svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  const presets = [5, 10, 20];

  async function onaction() {
    // The input is a percent; the domain API takes a fraction (percent / 100).
    await modal.plan.snooze(modal.part.id!, parseFloat(modal.fraction) / 100);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.snoozeplan_header({ name: modal.part.name })}
  {/snippet}
  <div class="flex items-center gap-2 mb-3">
    {#each presets as p}
      <Button
        size="xs"
        color="alternative"
        onclick={() => (modal.fraction = String(p))}
      >
        {p}%
      </Button>
    {/each}
  </div>
  <ButtonGroup class="w-full">
    <Input
      type="number"
      min="0"
      step="any"
      required
      autofocus
      bind:value={modal.fraction}
      placeholder={m.snoozeplan_percent()}
    />
    <InputAddon>%</InputAddon>
  </ButtonGroup>
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_snooze()} />
  {/snippet}
</Modal>
