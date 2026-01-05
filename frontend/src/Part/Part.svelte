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
  import { shopMode, user, updateSummary } from "../lib/store";
  import { shops, type Shop } from "../lib/shop";
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

  // Check if user can unregister this part from the current shop
  let canUnregister = $derived(
    $shopMode.active &&
      $shopMode.shop &&
      ($shopMode.shop.owner === $user?.id || part.owner === $user?.id),
  );

  // Fetch user's shops (only owned shops, not in shop mode)
  let userShops = $derived($shopMode.active ? [] : Object.values($shops));

  // Track which shops this part is registered to
  let partRegisteredShops = $state<number[]>([]);

  async function unregisterFromShop() {
    if (!$shopMode.shop) return;
    try {
      await $shopMode.shop.unregisterPart(part.id!);
      updateSummary();
    } catch (error) {
      console.error("Error unregistering part:", error);
    }
  }

  // Load which shops this part is registered to
  async function loadPartShops() {
    if (!part.id || userShops.length === 0) {
      partRegisteredShops = [];
      return;
    }

    try {
      const promises = userShops.map(async (shop) => {
        try {
          const partIds = await shop.getParts();
          // partIds is an array of numbers, not objects
          return partIds.includes(part.id!) ? shop.id : null;
        } catch {
          return null;
        }
      });

      const results = await Promise.all(promises);
      partRegisteredShops = results.filter((id): id is number => id !== null);
    } catch (error) {
      console.error("Error loading part shops:", error);
      partRegisteredShops = [];
    }
  }

  // Toggle registration of this part to a shop
  async function toggleShopRegistration(shop: Shop) {
    if (!part.id || !shop.id) return;

    const isRegistered = partRegisteredShops.includes(shop.id);

    try {
      if (isRegistered) {
        await shop.unregisterPart(part.id);
        partRegisteredShops = partRegisteredShops.filter(
          (id) => id !== shop.id,
        );
      } else {
        await shop.registerPart(part.id);
        partRegisteredShops = [...partRegisteredShops, shop.id];
      }
    } catch (error) {
      console.error("Error toggling shop registration:", error);
      // Reload to ensure UI reflects actual state
      await loadPartShops();
    }
  }

  // Load registered shops on mount
  onMount(() => {
    loadPartShops();
  });
</script>

<GearCard {part}>
  <Menu>
    {#if canUnregister}
      <DropdownItem onclick={unregisterFromShop}>
        Unregister from {$shopMode.shop?.name}
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

    <!-- Shop Registration Section -->
    {#if !$shopMode.active && $user?.id === part.owner && userShops.length > 0}
      <DropdownDivider />
      <div class="px-3 py-2">
        <p class="text-sm font-medium text-gray-900 dark:text-white">
          Register to Shops
        </p>
      </div>
      {#each userShops as shop}
        <DropdownItem class="flex items-center gap-2">
          <Checkbox
            checked={partRegisteredShops.includes(shop.id!)}
            onchange={() => toggleShopRegistration(shop)}
          />
          <span>{shop.name}</span>
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
