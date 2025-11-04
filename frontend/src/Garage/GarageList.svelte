<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import GarageCard from "./GarageCard.svelte";
  import type { Garage } from "../lib/garage";
  import { actions } from "../Widgets/Actions.svelte";
  import { user, enterGarage } from "../lib/store";
  import type { Snippet } from "svelte";

  interface Props {
    garages: Garage[];
    sub?: Snippet<[Garage]>;
    showEnterGarage?: boolean; // Only show "Enter Garage" if user has access
  }

  let { sub, garages, showEnterGarage = false }: Props = $props();
</script>

<div class="grid gap-4 grid-cols-1">
  {#each garages as garage}
    {@const isOwner = garage.owner === $user?.id}
    <GarageCard {garage} {isOwner} {sub}>
      {#if showEnterGarage}
        <DropdownItem onclick={() => enterGarage(garage.id!)}>
          Enter Garage
        </DropdownItem>
      {/if}
      {#if isOwner}
        <DropdownItem onclick={() => $actions.editGarage(garage)}>
          Edit Garage
        </DropdownItem>
        <DropdownItem onclick={() => $actions.deleteGarage(garage)}>
          Delete Garage
        </DropdownItem>
      {:else}
        <DropdownItem onclick={() => $actions.requestSubscription(garage)}>
          Request Subscription
        </DropdownItem>
      {/if}
    </GarageCard>
  {/each}
</div>

{#if garages.length === 0}
  <div class="py-12 text-center">
    <p class="text-gray-500 dark:text-gray-400">
      No garages found. Create your first garage to get started!
    </p>
  </div>
{/if}
