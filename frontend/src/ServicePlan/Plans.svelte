<script lang="ts">
  import { Button } from "flowbite-svelte";
  import { filterValues } from "../lib/mapable";
  import { plans } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import NewPlan from "./NewPlan.svelte";
  import PlanList from "./PlanList.svelte";

  let newPlan: () => void;

  $: planlist = filterValues(
    $plans,
    (p) => types[p.what].main == $category.main,
  );
</script>

<PlanList {planlist}>
  Serviceplans
  <Button size="sm" color="light" on:click={() => newPlan()}>add</Button> &NonBreakingSpace;
</PlanList>
<NewPlan bind:newPlan />
