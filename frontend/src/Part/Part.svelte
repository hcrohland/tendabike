<script lang="ts">
  import {
    Button,
    ButtonGroup,
    TabContent,
    TabPane,
  } from "@sveltestrap/sveltestrap";
  import ServiceList from "../Service/ServiceList.svelte";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";
  import PartHist from "./PartHist.svelte";
  import Subparts from "./Subparts.svelte";

  export let params: { id: number; what: number };

  let tab: number | string = "parts";

  $: part = $parts[params.id];
  $: attachees = filterValues($attachments, (a) => a.gear == part.id);
  $: planlist = plans_for_part_and_subtypes($attachments, $plans, part);
</script>

<GearCard {part} display>
  <ButtonGroup class="float-end">
    {#if part.disposed_at}
      <Button color="light" on:click={() => $actions.recoverPart(part)}>
        Recover gear
      </Button>
    {:else}
      <Button color="light" on:click={() => $actions.changePart(part)}>
        Change details
      </Button>
      {#if !part.isGear()}
        <Button color="light" on:click={() => $actions.attachPart(part)}>
          Attach part
        </Button>
      {/if}
    {/if}
  </ButtonGroup>
</GearCard>
<br />
<PartHist id={params.id} />
<TabContent on:tab={(e) => (tab = e.detail)}>
  {#if attachees.length > 0 || part.isGear()}
    <TabPane tabId="parts" active>
      <strong slot="tab">
        Attached Parts
        {#if tab == "parts" && part.isGear()}
          <Button
            size="sm"
            color="light"
            on:click={() => $actions.installPart(part)}
          >
            add
          </Button>
        {/if}
      </strong>
      <Subparts {part} {attachees} />
    </TabPane>
  {/if}
  <TabPane tabId="plans" active={!(attachees.length > 0 || part.isGear())}>
    <strong slot="tab">
      Service Plans
      <PlanBadge {planlist} />
      {#if tab == "plans"}
        <Button size="sm" color="light" on:click={() => $actions.newPlan(part)}>
          add
        </Button> &NonBreakingSpace;
      {/if}
    </strong>
    <PlanList {planlist} /><br />
  </TabPane>
  <TabPane tabId="service">
    <strong slot="tab">
      Service Logs
      {#if tab == "service"}
        <Button
          size="sm"
          color="light"
          on:click={() => $actions.newService(part)}
        >
          add
        </Button>
      {/if}
    </strong>
    <ServiceList {part} /><br />
  </TabPane>
</TabContent>
