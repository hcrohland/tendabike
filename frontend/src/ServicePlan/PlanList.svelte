<script lang="ts">
  import { Table } from "@sveltestrap/sveltestrap";
  import { ServicePlan } from "../lib/serviceplan";
  import PlanHeader from "./PlanHeader.svelte";
  import { by } from "../lib/mapable";
  import PlanBlock from "./PlanBlock.svelte";

  export let planlist: ServicePlan[];
</script>

<Table responsive hover>
  <thead>
    <PlanHeader />
  </thead>
  <tbody>
    {#each planlist
      .sort(by("id"))
      .sort(by("part"))
      .sort(by("what", true)) as plan}
      <PlanBlock {plan} />
    {/each}
  </tbody>
</Table>
