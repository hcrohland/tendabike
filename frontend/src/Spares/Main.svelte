<script lang="ts">
  import { TabContent, TabPane } from "@sveltestrap/sveltestrap";
  import Spares from "./Spares.svelte";
  import Plans from "./Plans.svelte";
</script>

<TabContent>
  <TabPane tabId="parts" active>
    <strong slot="tab"> Parts </strong>
    <Spares />
  </TabPane>
  <TabPane tabId="plans">
    <strong slot="tab"> Serviceplans </strong>
    <Plans />
  </TabPane>
</TabContent>
