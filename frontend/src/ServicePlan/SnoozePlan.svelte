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

  // The shared submit path: run the snooze with a fraction and dismiss.
  // Presets and the main button both go through here.
  async function snooze(fraction: number) {
    await modal.plan.snooze(modal.part.id!, fraction);
    modal.open = false;
  }

  // The form-submit path (main button) reads the typed percent from the input;
  // the input is a percent, the domain API takes a fraction (percent / 100).
  function onaction() {
    return snooze(parseFloat(modal.fraction) / 100);
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.snoozeplan_header({ name: modal.part.name })}
  {/snippet}
  <ButtonGroup class="w-full">
    <Input
      type="number"
      min="0"
      step="any"
      required
      autofocus
      class="w-20!"
      bind:value={modal.fraction}
      placeholder={m.snoozeplan_percent()}
    />
    <InputAddon>%</InputAddon>
    {#each presets as p}
      <Button color="secondary" onclick={() => snooze(p / 100)}>
        {p}%
      </Button>
    {/each}
  </ButtonGroup>
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_snooze()} />
  {/snippet}
</Modal>
