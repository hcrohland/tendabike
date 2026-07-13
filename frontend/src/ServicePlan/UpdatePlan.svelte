<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import { category, types } from "../lib/types";
  import PlanModal from "./PlanModal.svelte";
  import { m } from "../../paraglide/messages";

  let header: string = $state("");
  let modal: { start: (p: ServicePlan) => void };

  async function safePlan(newplan: ServicePlan) {
    await newplan.update();
  }

  export function start(p: ServicePlan) {
    if (p.part) {
      let part = $parts[p.part];
      if (part.isGear() && p.hook != null) {
        header = m.updateplan_header_hook_part({
          hook: types[p.what].human_name(p.hook),
          name: part.name,
        });
      } else {
        header = m.updateplan_header_part({ name: part.name });
      }
    } else {
      header = m.updateplan_header_generic({
        hook: types[p.what].human_name(p.hook),
        any: $category.localizedAnyDative(),
      });
    }
    modal.start(new ServicePlan(p));
  }
</script>

<PlanModal bind:this={modal} {safePlan} no_gear>
  {header}
</PlanModal>
