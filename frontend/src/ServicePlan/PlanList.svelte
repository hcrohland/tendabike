<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanRow from "./PlanRow.svelte";

  export let planlist: ServicePlan[];
</script>

<Table responsive hover>
  <thead>
    <PlanHeader />
  </thead>
  <tbody>
    {#each planlist as plan (plan.id)}
      <PlanRow {plan} />
    {/each}
  </tbody>
</Table>
