<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import { onMount } from "svelte";
  import GarageList from "./GarageList.svelte";
  import Subscriptions from "./Subscriptions.svelte";
  import { garages } from "../lib/garage";
  import { actions } from "../Widgets/Actions.svelte";

  let activeTab = $state<string>("my-subscriptions");

  // Get user's garages from the store
  let myGarages = $derived(Object.values($garages));

  onMount(() => {
    // Garages are loaded via the summary endpoint automatically
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Garages</h1>
    <Button onclick={() => $actions.createGarage()}>Create Garage</Button>
  </div>

  <Tabs style="underline" bind:selected={activeTab}>
    <TabItem key="my-subscriptions" title="My Subscriptions" open>
      <div class="py-4">
        <Subscriptions showMySubscriptions={true} />
      </div>
    </TabItem>

    <TabItem key="my-garages" title="My Garages">
      <div class="py-4 space-y-8">
        {#if myGarages.length === 0}
          <div class="py-12 text-center">
            <p class="mb-4 text-gray-500 dark:text-gray-400">
              You don't have any garages yet.
            </p>
            <Button onclick={() => $actions.createGarage()}>
              Create Your First Garage
            </Button>
          </div>
        {:else}
          <!-- Garage Cards -->
          <div>
            <h2
              class="mb-4 text-xl font-semibold text-gray-900 dark:text-white"
            >
              Your Garages
            </h2>
            <GarageList garages={myGarages} />
          </div>

          <!-- Pending Subscription Requests -->
          <div>
            <h2
              class="mb-4 text-xl font-semibold text-gray-900 dark:text-white"
            >
              Pending Subscription Requests
            </h2>
            {#each myGarages as garage}
              <div class="mb-6">
                <h3
                  class="mb-3 text-lg font-medium text-gray-800 dark:text-gray-200"
                >
                  {garage.name}
                </h3>
                <Subscriptions
                  garageId={garage.id}
                  showMySubscriptions={false}
                />
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
