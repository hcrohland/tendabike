<script lang="ts">
  import { Badge, Button, TabContent, TabPane } from "@sveltestrap/sveltestrap";
  import Spares from "./Spares.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { plans } from "../lib/serviceplan";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  let newPlan: () => void;

  let alerts = { alert: 0, warn: 0 };

  let tab: string | number = "parts";
</script>

<TabContent on:tab={(e) => (tab = e.detail)}>
  <TabPane tabId="parts" active>
    <strong slot="tab"> Parts </strong>
    <Spares />
  </TabPane>
  <TabPane tabId="plans">
    <strong slot="tab">
      Serviceplans
      {#if alerts.alert > 0}
        <Badge color="danger">{alerts.alert}</Badge>
      {:else if alerts.warn > 0}
        <Badge color="warning">{alerts.warn}</Badge>
      {/if}
      {#if tab == "plans"}
        <Button size="sm" color="light" on:click={() => newPlan()}>add</Button> &NonBreakingSpace;
      {/if}
    </strong>
    <PlanList planlist={Object.values($plans)} bind:alerts />
  </TabPane>
</TabContent>
<NewPlan bind:newPlan />
