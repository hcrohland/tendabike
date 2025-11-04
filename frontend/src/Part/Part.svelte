<script lang="ts">
  import {
    Tabs,
    TabItem,
    DropdownItem,
    Checkbox,
    DropdownDivider,
  } from "flowbite-svelte";
  import ServiceList from "../Service/ServiceList.svelte";
  import PlanBadge from "../ServicePlan/PlanBadge.svelte";
  import PlanList from "../ServicePlan/PlanList.svelte";
  import { attachments } from "../lib/attachment";
  import { filterValues } from "../lib/mapable";
  import { Part, parts } from "../lib/part";
  import { plans, plans_for_part_and_subtypes } from "../lib/serviceplan";
  import GearCard from "./GearCard.svelte";
  import Subparts from "./Subparts.svelte";
  import PartHist from "./PartHist.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import XsButton from "../Widgets/XsButton.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import { pop } from "svelte-spa-router";
  import { garageMode, user, updateSummary } from "../lib/store";
  import { garages, type Garage } from "../lib/garage";
  import { onMount } from "svelte";

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

  // Check if user can unregister this part from the current garage
  let canUnregister = $derived(
    $garageMode.active &&
      $garageMode.garage &&
      ($garageMode.garage.owner === $user?.id || part.owner === $user?.id),
  );

  // Fetch user's garages (only owned garages, not in garage mode)
  let userGarages = $derived($garageMode.active ? [] : Object.values($garages));

  // Track which garages this part is registered to
  let partRegisteredGarages = $state<number[]>([]);

  async function unregisterFromGarage() {
    if (!$garageMode.garage) return;
    try {
      await $garageMode.garage.unregisterPart(part.id!);
      updateSummary();
    } catch (error) {
      console.error("Error unregistering part:", error);
    }
  }

  // Load which garages this part is registered to
  async function loadPartGarages() {
    if (!part.id || userGarages.length === 0) {
      partRegisteredGarages = [];
      return;
    }

    try {
      const promises = userGarages.map(async (garage) => {
        try {
          const partIds = await garage.getParts();
          // partIds is an array of numbers, not objects
          return partIds.includes(part.id!) ? garage.id : null;
        } catch {
          return null;
        }
      });

      const results = await Promise.all(promises);
      partRegisteredGarages = results.filter((id): id is number => id !== null);
    } catch (error) {
      console.error("Error loading part garages:", error);
      partRegisteredGarages = [];
    }
  }

  // Toggle registration of this part to a garage
  async function toggleGarageRegistration(garage: Garage) {
    if (!part.id || !garage.id) return;

    const isRegistered = partRegisteredGarages.includes(garage.id);

    try {
      if (isRegistered) {
        await garage.unregisterPart(part.id);
        partRegisteredGarages = partRegisteredGarages.filter(
          (id) => id !== garage.id,
        );
      } else {
        await garage.registerPart(part.id);
        partRegisteredGarages = [...partRegisteredGarages, garage.id];
      }
    } catch (error) {
      console.error("Error toggling garage registration:", error);
      // Reload to ensure UI reflects actual state
      await loadPartGarages();
    }
  }

  // Load registered garages on mount
  onMount(() => {
    loadPartGarages();
  });
</script>

<GearCard {part}>
  <Menu>
    {#if canUnregister}
      <DropdownItem onclick={unregisterFromGarage}>
        Unregister from {$garageMode.garage?.name}
      </DropdownItem>
    {/if}
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

    <!-- Garage Registration Section -->
    {#if !$garageMode.active && $user?.id === part.owner && userGarages.length > 0}
      <DropdownDivider />
      <div class="px-3 py-2">
        <p class="text-sm font-medium text-gray-900 dark:text-white">
          Register to Garages
        </p>
      </div>
      {#each userGarages as garage}
        <DropdownItem class="flex items-center gap-2">
          <Checkbox
            checked={partRegisteredGarages.includes(garage.id!)}
            onchange={() => toggleGarageRegistration(garage)}
          />
          <span>{garage.name}</span>
        </DropdownItem>
      {/each}
    {/if}
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
