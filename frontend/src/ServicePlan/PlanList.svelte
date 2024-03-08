<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import { parts } from "../lib/part";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanRow from "./PlanRow.svelte";

  export let planlist: ServicePlan[];
  export let part_id: number | undefined;
</script>

<Table responsive hover>
  <thead>
    <PlanHeader />
  </thead>
  <tbody>
    {#each planlist as plan}
      {@const name =
        part_id == plan.part ? "" : "for " + $parts[plan.part].partLink()}
      <PlanRow {plan} {name} />
    {/each}
  </tbody>
</Table>
