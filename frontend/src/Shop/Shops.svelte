<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import ShopList from "./ShopList.svelte";
  import Subscriptions from "./Subscriptions.svelte";
  import { Shop, shops } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user } from "../lib/store";

  let activeTab = $state<string>("my-subscriptions");

  // Get all user's shops from the store (owned + subscribed)
  let myShops = $derived(
    Object.values($shops).filter((g) => g.owner === $user?.id),
  );
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Shops</h1>
    <Button onclick={() => $actions.createShop()}>Create Shop</Button>
  </div>

  <Tabs style="underline" bind:selected={activeTab}>
    <TabItem key="my-subscriptions" title="My Subscriptions">
      <div class="py-4">
        <Subscriptions showMySubscriptions={true} />
      </div>
    </TabItem>

    <TabItem key="my-shops" title="My Shops" open={myShops.length > 0}>
      <div class="py-4 space-y-8">
        {#if myShops.length === 0}
          <div class="py-12 text-center">
            <p class="mb-4 text-gray-500 dark:text-gray-400">
              You don't have any shops yet.
            </p>
            <Button onclick={() => $actions.createShop()}>
              Create Your First Shop
            </Button>
          </div>
        {:else}
          <!-- Shop Cards -->
          <div>
            <h2
              class="mb-4 text-xl font-semibold text-gray-900 dark:text-white"
            >
              Your Shops
            </h2>
            <ShopList shops={myShops} showEnterShop={true}>
              {#snippet sub(shop: Shop)}
                <Subscriptions shopId={shop.id} showMySubscriptions={false} />
              {/snippet}
            </ShopList>
          </div>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
