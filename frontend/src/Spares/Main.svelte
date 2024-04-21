<script lang="ts">
  import { TabContent, TabPane } from "@sveltestrap/sveltestrap";
  import Spares from "./Spares.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { plans } from "../lib/serviceplan";
</script>

<TabContent>
  <TabPane tabId="parts" active>
    <strong slot="tab"> Parts </strong>
    <Spares />
  </TabPane>
  <TabPane tabId="plans">
    <strong slot="tab"> Serviceplans </strong>
    <PlanList planlist={Object.values($plans)} />
  </TabPane>
</TabContent>
