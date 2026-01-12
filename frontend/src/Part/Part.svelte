<script lang="ts">
  import { Tabs, TabItem, DropdownItem } from "flowbite-svelte";
  import ServiceList from "../Service/ServiceList.svelte";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import { parts } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";
  import Subparts from "./Subparts.svelte";
  import PartHist from "./PartHist.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { pop } from "svelte-spa-router";
  import ShopRegistration from "../Shop/ShopRegistration.svelte";

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

<GearCard {part}>
  <Menu>
    {#if part.disposed_at}
      <DropdownItem onclick={() => $actions.recoverPart(part)}>
        Recover gear
      </DropdownItem>
    {:else}
      {#if !part.isGear()}
        <DropdownItem onclick={() => $actions.attachPart(part)}>
          Attach
        </DropdownItem>
      {/if}
      <DropdownItem onclick={() => $actions.disposePart(part, last_attachment)}>
        {#if last_attachment?.isAttached()}
          Detach
        {:else}
          Dispose
        {/if}
      </DropdownItem>
      <DropdownItem onclick={() => $actions.changePart(part)}>
        Change details
      </DropdownItem>
    {/if}
    {#if !part.isGear() && part.attachments($attachments).length == 0}
      <DropdownItem
        onclick={() => {
          $actions.deletePart(part);
          pop();
        }}
      >
        Delete
      </DropdownItem>
    {/if}

    <ShopRegistration {part} {last_attachment} />
  </Menu>
</GearCard>
<br />
<PartHist {id} />
<Tabs bind:selected={tab}>
  {#if attachees.length > 0 || part.isGear()}
    <TabItem key="parts">
      {#snippet titleSlot()}
        Attached Parts
        {#if tab == "parts"}
          <XsButton onclick={() => $actions.installPart(part)}>add</XsButton>
        {/if}
      {/snippet}
      <Subparts {part} {attachees} />
    </TabItem>
  {/if}
  <TabItem key="plans">
    {#snippet titleSlot()}
      Service Plans
      <PlanBadge {planlist} />
      {#if tab == "plans"}
        <XsButton onclick={() => $actions.newPlan(part)}>add</XsButton>
      {/if}
    {/snippet}
    <PlanList {planlist} /><br />
  </TabItem>
  <TabItem key="services">
    {#snippet titleSlot()}
      Service Logs
      {#if tab == "services"}
        <XsButton onclick={() => $actions.newService(part)}>add</XsButton>
      {/if}
    {/snippet}
    <ServiceList {part} /><br />
  </TabItem>
</Tabs>
