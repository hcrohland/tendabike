<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import ShopList from "./ShopList.svelte";
  import Subscriptions from "./Subscriptions.svelte";
  import { Shop, shops } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user, users } from "../lib/user";
  import { filterValues } from "../lib/mapable";
  import ShopSubscriptions from "./ShopSubscriptions.svelte";

  let activeTab = $state<string>("my-subscriptions");

  // Get all user's shops from the store (owned + subscribed)
  let myShops = $derived(filterValues($shops, (g) => g.owner === $user?.id));
</script>

<div class="space-y-6">
  <Tabs style="underline" bind:selected={activeTab}>
    <TabItem key="my-subscriptions" title="My Subscriptions">
      <div class="py-4">
        <Subscriptions />
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
            <ShopList shops={myShops} users={$users}>
              {#snippet sub(shop: Shop)}
                <ShopSubscriptions shopId={shop.id!} />
              {/snippet}
            </ShopList>
          </div>
          <Button onclick={() => $actions.createShop()}>Create Shop</Button>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
