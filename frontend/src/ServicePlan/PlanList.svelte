<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan, alerts_for_plans } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanBlock from "./PlanBlock.svelte";
  import { parts } from "../lib/part";
  import { services } from "../lib/service";
  import { usages } from "../lib/usage";
  import { attachments } from "../lib/attachment";

  export let planlist: ServicePlan[];
  export let alerts;

  alerts = alerts_for_plans(planlist, $parts, $services, $usages, $attachments);

  function cmp(p: ServicePlan, q: ServicePlan) {
    let res;
    if (p.what != q.what) {
      res = p.what < q.what;
    } else if (p.hook != q.hook) {
      res = p.what < q.what;
    } else if (p.part != q.part) {
      res = p.part! < q.part!;
    } else {
      res = p.id! < q.id!;
    }
    return res ? -1 : 1;
  }
</script>

<Table responsive hover>
  <thead>
    <PlanHeader />
  </thead>
  <tbody>
    {#each planlist.sort(cmp) as plan}
      <PlanBlock {plan} />
    {/each}
  </tbody>
</Table>
