<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanBlock from "./PlanBlock.svelte";

  export let planlist: ServicePlan[];

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
    <PlanHeader><slot /></PlanHeader>
  </thead>
  <tbody>
    {#each planlist.sort(cmp) as plan}
      <PlanBlock {plan} />
    {/each}
  </tbody>
</Table>
