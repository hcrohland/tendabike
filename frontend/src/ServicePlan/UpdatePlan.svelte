<script module lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import { getCategory, types } from "../lib/types";
  import { start as startPlan } from "./PlanModal.svelte";
  import { m } from "../../paraglide/messages";

  const config = $state<{
    header: string;
  }>({
    header: "",
  });

  export function start(p: ServicePlan) {
    if (p.part) {
      let part = parts[p.part];
      if (part.isGear() && p.hook != null) {
        config.header = m.updateplan_header_hook_part({
          hook: types[p.what].human_name(p.hook),
          name: part.name,
        });
      } else {
        config.header = m.updateplan_header_part({ name: part.name });
      }
    } else {
      config.header = m.updateplan_header_generic({
        hook: types[p.what].human_name(p.hook),
        any: getCategory()!.localizedAnyDative(),
      });
    }
    startPlan(new ServicePlan(p));
  }
</script>

<script lang="ts">
  import PlanModal from "./PlanModal.svelte";

  async function safePlan(newplan: ServicePlan) {
    await newplan.update();
  }
</script>

<PlanModal {safePlan} no_gear>
  {config.header}
</PlanModal>
