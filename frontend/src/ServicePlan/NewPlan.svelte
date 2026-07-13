<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { Part } from "../lib/part";
  import PlanModal from "./PlanModal.svelte";
  import { m } from "../../paraglide/messages";

  let partname = $state("");
  let modal: { start: (p: ServicePlan) => void };
  let no_gear = $state(false);
  let plan = $state(new ServicePlan({}));

  async function safePlan(newplan: ServicePlan) {
    await newplan.create();
  }

  export function start(p: Part) {
    partname = p.name;
    if (p && !p.isGear()) {
      plan = new ServicePlan({ part: p.id, what: p.what, hook: null });
      no_gear = true;
    } else {
      plan = new ServicePlan({ part: p?.id });
      no_gear = false;
    }
    modal.start(plan);
  }
</script>

<PlanModal bind:this={modal} {safePlan} {no_gear}>
  {m.newplan_header_part({ name: no_gear ? partname : "" })}
</PlanModal>
