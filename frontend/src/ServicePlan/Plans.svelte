<script lang="ts">
  import { Button } from "@sveltestrap/sveltestrap";
  import { plans } from "../lib/serviceplan";
  import PlanList from "./PlanList.svelte";
  import NewPlan from "./NewPlan.svelte";
  import { filterValues } from "../lib/mapable";
  import { category } from "../lib/types";

  let newPlan: () => void;

  $: planlist = filterValues($plans, (p) => p.what == $category.main);
</script>

<PlanList {planlist}>
  Serviceplans
  <Button size="sm" color="light" on:click={() => newPlan()}>add</Button> &NonBreakingSpace;
</PlanList>
<NewPlan bind:newPlan />
