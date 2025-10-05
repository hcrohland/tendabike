<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { Part } from "../lib/part";
  import PlanModal from "./PlanModal.svelte";

  let part: Part | undefined = $state();
  let modal: { start: (p: ServicePlan) => void };
  let no_gear = $state(false);
  let plan = $state(new ServicePlan({}));

  async function safePlan(newplan: ServicePlan) {
    await newplan.create();
  }

  export function start(p: Part) {
    if (p && !p.isGear()) {
      plan = new ServicePlan({ part: p.id, what: p.what, hook: null });
      no_gear = true;
    } else {
      plan = new ServicePlan({ part: p?.id });
    }
    modal.start(plan);
  }
</script>

<PlanModal bind:this={modal} {safePlan} {no_gear}>
  New service plan for
  {#if no_gear}
    {part!.name}
  {/if}
</PlanModal>
