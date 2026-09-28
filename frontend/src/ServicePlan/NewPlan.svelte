<script module lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { Part } from "../lib/part";
  import { start as startPlan } from "./PlanModal.svelte";

  const config = $state<{
    partname: string;
    no_gear: boolean;
    plan: ServicePlan;
  }>({
    partname: "",
    no_gear: false,
    plan: new ServicePlan({}),
  });

  export function start(p: Part) {
    config.partname = p.name;
    if (p && !p.isGear()) {
      config.plan = new ServicePlan({ part: p.id, what: p.what, hook: null });
      config.no_gear = true;
    } else {
      config.plan = new ServicePlan({ part: p?.id });
      config.no_gear = false;
    }
    startPlan(config.plan);
  }
</script>

<script lang="ts">
  import PlanModal from "./PlanModal.svelte";
  import { m } from "../../paraglide/messages";

  async function safePlan(newplan: ServicePlan) {
    await newplan.create();
  }
</script>

<PlanModal {safePlan} no_gear={config.no_gear}>
  {m.newplan_header_part({ name: config.no_gear ? config.partname : "" })}
</PlanModal>
