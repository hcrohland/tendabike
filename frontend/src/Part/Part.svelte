<script lang="ts">
  import { Tabs, TabItem, DropdownItem } from "flowbite-svelte";
  import ServiceList from "../Service/ServiceList.svelte";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { attachments } from "../lib/attachment";
  import { stateValues } from "../lib/mapable.svelte";
  import { parts } from "../lib/part";
  import GearCard from "./GearCard.svelte";
  import Subparts from "./Subparts.svelte";
  import PartHist from "./PartHist.svelte";
  import NoteList from "./NoteList.svelte";
  import { getActions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { pop } from "svelte-spa-router";
  import ShopRegistration from "../Shop/ShopRegistration.svelte";
  import { m } from "../../paraglide/messages";

  interface Props {
    id: number;
  }

  let { id }: Props = $props();

  let part = $derived(parts[id]);
  let attachees = $derived(
    stateValues(attachments).filter((a) => a.gear == part.id),
  );
  let last_attachment = $derived(part.attachments(attachments).at(0));

  let tab = $state("");
</script>

<GearCard {part}>
  <div class="float-end h6 mb-0">
    <Menu>
      {#if part.disposed_at}
        <DropdownItem onclick={() => getActions()!.recoverPart(part)}>
          {m.part_recover_gear()}
        </DropdownItem>
      {:else}
        {#if !part.isGear()}
          <DropdownItem onclick={() => getActions()!.attachPart(part)}>
            {m.action_attach()}
          </DropdownItem>
        {/if}
        <DropdownItem
          onclick={() => getActions()!.disposePart(part, last_attachment)}
        >
          {#if last_attachment?.isAttached()}
            {m.action_detach()}
          {:else}
            {m.action_dispose()}
          {/if}
        </DropdownItem>
        <DropdownItem onclick={() => getActions()!.changePart(part)}>
          {m.part_change_details()}
        </DropdownItem>
      {/if}
      {#if !part.isGear() && part.attachments(attachments).length == 0}
        <DropdownItem
          onclick={() => {
            getActions()!.deletePart(part);
            pop();
          }}
        >
          {m.action_delete()}
        </DropdownItem>
      {/if}

      <ShopRegistration {part} {last_attachment} />
    </Menu>
  </div>
</GearCard>
<br />
<PartHist {id} />
<Tabs bind:selected={tab} classes={{ content: "m-0 p-0 md:m-2 md:p-2" }}>
  {#if attachees.length > 0 || part.isGear()}
    <TabItem key="parts">
      {#snippet titleSlot()}
        {m.part_tab_attached_parts()}
        {#if tab == "parts"}
          <XsButton onclick={() => getActions()!.installPart(part)}>
            {m.partcard_add()}
          </XsButton>
        {/if}
      {/snippet}
      <Subparts {part} {attachees} />
    </TabItem>
  {/if}
  <TabItem key="plans">
    {#snippet titleSlot()}
      {m.part_tab_service_plans()}
      <PlanBadge {part} />
      {#if tab == "plans"}
        <XsButton onclick={() => getActions()!.newPlan(part)}>
          {m.partcard_add()}
        </XsButton>
      {/if}
    {/snippet}
    <PlanList {part} />
  </TabItem>
  <TabItem key="services">
    {#snippet titleSlot()}
      {m.part_tab_service_logs()}
      {#if tab == "services"}
        <XsButton onclick={() => getActions()!.newService(part)}>
          {m.partcard_add()}
        </XsButton>
      {/if}
    {/snippet}
    <ServiceList {part} />
  </TabItem>
  <TabItem key="notes">
    {#snippet titleSlot()}
      {m.part_tab_notes()}
      {#if tab == "notes"}
        <XsButton onclick={() => getActions()!.newNote(part)}>
          {m.partcard_add()}
        </XsButton>
      {/if}
    {/snippet}
    <NoteList {part} />
  </TabItem>
</Tabs>
