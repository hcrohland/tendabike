<script lang="ts">
  import { Table, TableBody, TableHead } from "flowbite-svelte";
  import { ServicePlan } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanBlock from "./PlanBlock.svelte";

  interface Props {
    planlist: ServicePlan[];
    children?: import("svelte").Snippet;
  }

  let { planlist, children }: Props = $props();

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

<Table border={false} striped hoverable>
  <TableHead>
    <PlanHeader>{@render children?.()}</PlanHeader>
  </TableHead>
  <TableBody>
    {#each planlist.sort(cmp) as plan}
      <PlanBlock {plan} />
    {/each}
  </TableBody>
</Table>
