<script lang="ts">
  import { DropdownItem, Tabs, TabItem } from "flowbite-svelte";
  // import ServiceList from "../Service/ServiceList.svelte";
  // import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  // import PlanList from "../ServicePlan/PlanList.svelte";
  // import { actions } from "../Widgets/Actions.svelte";
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";
  // import PartHist from "./PartHist.svelte";
  // import Subparts from "./Subparts.svelte";
  // import AddButton from "./AddButton.svelte";
  // import Menu from "../Widgets/Menu.svelte";

  interface Props {
    id: number;
  }

  let { id }: Props = $props();

  let tab = $state("parts");

  let part = $derived($parts[id]);
  let attachees = $derived(
    filterValues($attachments, (a) => a.gear == part.id),
  );
  let last_attachment = $derived(part.attachments($attachments).at(0));
  let planlist = $derived(
    plans_for_part_and_subtypes($attachments, $plans, part),
  );
</script>

<GearCard {part}></GearCard>
<br />
<!-- <PartHist id={params.id} /> -->
<Tabs bind:selected={tab}>
  {#if attachees.length > 0 || part.isGear()}
    <TabItem key="parts">
      {#snippet titleSlot()}
        <strong>
          Attached Parts
          <!-- <AddButton {part} {tab} here="parts" /> -->
        </strong>
      {/snippet}
      <!-- <Subparts {part} {attachees} /> -->
    </TabItem>
  {/if}
  <TabItem key="plans">
    {#snippet titleSlot()}
      <strong>
        Service Plans
        <!-- <PlanBadge {planlist} /> -->
        <!-- <AddButton {part} {tab} here="plans" /> -->
      </strong>
    {/snippet}
    <!-- <PlanList {planlist} /><br /> -->
  </TabItem>
  <TabItem key="services">
    {#snippet titleSlot()}
      <strong>
        Service Logs
        <!-- <AddButton {part} {tab} here="services" /> -->
      </strong>
    {/snippet}
    <!-- <ServiceList {part} /><br /> -->
  </TabItem>
</Tabs>
