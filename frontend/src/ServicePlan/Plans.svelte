<script lang="ts">
  import { filterValues } from "../lib/mapable";
  import { plans } from "../lib/serviceplan";
  import { category, types } from "../lib/types";
  import NewPlan from "./NewPlan.svelte";
  import PlanList from "./PlanList.svelte";

  let planlist = $derived(
    filterValues($plans, (p) => types[p.what].main == $category.main),
  );
</script>

<PlanList {planlist}>
  Serviceplans
  <NewPlan />
</PlanList>
