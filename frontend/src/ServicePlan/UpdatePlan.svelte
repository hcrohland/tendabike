<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import { plans, ServicePlan } from "../lib/serviceplan";
  import { parts, Part } from "../lib/part";
  import { category, types } from "../lib/types";
  import PlanModal from "./PlanModal.svelte";

  interface Props {
    plan: ServicePlan;
  }

  let title: string = $state("");

  let { plan = $bindable() }: Props = $props();
  let open = $state(false);

  async function safePlan(newplan: ServicePlan) {
    await newplan.update();
    open = false;
  }

  const updatePlan = (p: ServicePlan) => {
    if (p.part) {
      let part = $parts[p.part];
      if (part.isGear() && plan.hook != null) {
        title = types[plan.what].human_name(plan.hook);
        title += types[plan.what].human_name(plan.hook) + " of ";
      }
      title += part.name;
    } else {
      title =
        types[plan.what].human_name(plan.hook) +
        " of any " +
        $category.name.toLocaleLowerCase();
    }
    plan = new ServicePlan(p);
    open = true;
  };
</script>

<DropdownItem onclick={() => updatePlan(plan)}>Change ServicePlan</DropdownItem>
<PlanModal bind:open {safePlan} {plan} no_gear>
  Update service plan for {title}
</PlanModal>
