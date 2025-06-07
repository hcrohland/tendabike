<script lang="ts">
  import { DropdownItem, TabContent, TabPane } from "@sveltestrap/sveltestrap";
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
  import AddButton from "./AddButton.svelte";
  import Menu from "../Widgets/Menu.svelte";

  export let params: { id: number };

  let tab: number | string = "parts";

  $: part = $parts[params.id];
  $: attachees = filterValues($attachments, (a) => a.gear == part.id);
  $: last_attachment = part.attachments($attachments).at(0);
  $: planlist = plans_for_part_and_subtypes($attachments, $plans, part);
</script>

<GearCard {part} display>
  <Menu>
    {#if part.disposed_at}
      <DropdownItem color="light" on:click={() => $actions.recoverPart(part)}>
        Recover gear
      </DropdownItem>
    {:else}
      {#if !part.isGear()}
        <DropdownItem color="light" on:click={() => $actions.attachPart(part)}>
          Attach
        </DropdownItem>
      {/if}
      <DropdownItem
        color="light"
        on:click={() => $actions.disposePart(part, last_attachment)}
      >
        {#if last_attachment?.isAttached()}
          Detach
        {:else}
          Dispose
        {/if}
      </DropdownItem>
      <DropdownItem color="light" on:click={() => $actions.changePart(part)}>
        Change details
      </DropdownItem>
    {/if}
  </Menu>
</GearCard>
<br />
<PartHist id={params.id} />
<TabContent on:tab={(e) => (tab = e.detail)}>
  {#if attachees.length > 0 || part.isGear()}
    <TabPane tabId="parts" active>
      <strong slot="tab">
        Attached Parts
        <AddButton {part} {tab} here="parts" />
      </strong>
      <Subparts {part} {attachees} />
    </TabPane>
  {/if}
  <TabPane tabId="plans">
    <strong slot="tab">
      Service Plans
      <PlanBadge {planlist} />
      <AddButton {part} {tab} here="plans" />
    </strong>
    <PlanList {planlist} /><br />
  </TabPane>
  <TabPane tabId="services">
    <strong slot="tab">
      Service Logs
      <AddButton {part} {tab} here="services" />
    </strong>
    <ServiceList {part} /><br />
  </TabPane>
</TabContent>
