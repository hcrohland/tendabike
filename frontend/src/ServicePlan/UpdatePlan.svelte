<script lang="ts">
  import { ServicePlan } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import { category, types } from "../lib/types";
  import PlanModal from "./PlanModal.svelte";

  let title: string = $state("");
  let modal: { start: (p: ServicePlan) => void };

  async function safePlan(newplan: ServicePlan) {
    await newplan.update();
  }

  export function start(p: ServicePlan) {
    if (p.part) {
      let part = $parts[p.part];
      if (part.isGear() && p.hook != null) {
        title += types[p.what].human_name(p.hook) + " of ";
      }
      title += part.name;
    } else {
      title =
        types[p.what].human_name(p.hook) +
        " of any " +
        $category.name.toLocaleLowerCase();
    }
    modal.start(new ServicePlan(p));
  }
</script>

<PlanModal bind:this={modal} {safePlan} no_gear>
  Update service plan for {title}
</PlanModal>
