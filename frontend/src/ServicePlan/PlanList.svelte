<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import PlanRow from "./PlanRow.svelte";
  import { by } from "../lib/mapable";

  export let planlist: ServicePlan[];
</script>

<Table responsive hover>
  <thead>
    <PlanHeader />
  </thead>
  <tbody>
    {#each planlist.sort(by("name", true)) as plan}
      <PlanRow {plan} />
    {/each}
  </tbody>
</Table>
