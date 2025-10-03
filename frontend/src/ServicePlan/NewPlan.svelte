<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";
  import { Part } from "../lib/part";
  import PlanModal from "./PlanModal.svelte";

  interface Props {
    part?: Part | undefined;
  }

  let { part = undefined }: Props = $props();

  let open = $state(false);
  let no_gear = $state(false);
  let plan = $state(new ServicePlan({}));

  async function safePlan(newplan: ServicePlan) {
    await newplan.create();
    open = false;
  }

  function newPlan() {
    if (part && !part.isGear()) {
      plan = new ServicePlan({ part: part.id, what: part.what, hook: null });
      no_gear = true;
    } else {
      plan = new ServicePlan({ part: part?.id });
      no_gear = false;
    }
    open = true;
  }
</script>

<Button
  size="xs"
  color="alternative"
  class="p-1 cursor-pointer"
  onclick={newPlan}
>
  add
</Button>
<PlanModal {safePlan} {plan} bind:open {no_gear}>
  New service plan for
  {#if no_gear}
    {part!.name}
  {/if}
</PlanModal>
