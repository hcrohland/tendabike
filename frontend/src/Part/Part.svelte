<script lang="ts">
  import { Tabs, TabItem } from "flowbite-svelte";
  import ServiceList from "../Service/ServiceList.svelte";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";
  import NewPlan from "../ServicePlan/NewPlan.svelte";
  import Subparts from "./Subparts.svelte";
  import PartHist from "./PartHist.svelte";
  // import AddButton from "./AddButton.svelte";

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
<PartHist {id} />
<Tabs bind:selected={tab}>
  {#if attachees.length > 0 || part.isGear()}
    <TabItem key="parts">
      {#snippet titleSlot()}
        Attached Parts
        <!-- <AddButton {part} {tab} here="parts" /> -->
      {/snippet}
      <Subparts {part} {attachees} />
    </TabItem>
  {/if}
  <TabItem key="plans">
    {#snippet titleSlot()}
      Service Plans
      <PlanBadge {planlist} />
      {#if tab == "plans"}
        <NewPlan {part} />
      {/if}
    {/snippet}
    <PlanList {planlist} /><br />
  </TabItem>
  <TabItem key="services">
    {#snippet titleSlot()}
      Service Logs
      <!-- <AddButton {part} {tab} here="services" /> -->
    {/snippet}
    <ServiceList {part} /><br />
  </TabItem>
</Tabs>
