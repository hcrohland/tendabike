<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { plans } from "../lib/serviceplan";
  import PlanList from "./PlanList.svelte";
  import NewPlan from "./NewPlan.svelte";

  let newPlan: () => void;
</script>

<PlanList planlist={Object.values($plans)}>
  Serviceplans
  <Button size="sm" color="light" on:click={() => newPlan()}>add</Button> &NonBreakingSpace;
</PlanList>
<NewPlan bind:newPlan />
