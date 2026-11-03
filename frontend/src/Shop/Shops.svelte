<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";

  import ShopList from "./ShopList.svelte";
  import Subscriptions from "./Subscriptions.svelte";
  import { Shop, shops } from "../lib/shop";
  import { createShop } from "./ShopModal.svelte";
  import { getUser, users } from "../lib/user";
  import { stateValues } from "../lib/mapable.svelte";
  import ShopSubscriptions from "./ShopSubscriptions.svelte";

  /// The /shops page accepts a tab selector in the route (a notification can
  /// deep-link to the correct tab); it defaults to the subscriptions tab when
  /// none is supplied (or the selector is not a known tab).
  let { query }: { query?: { tab?: string } } = $props();

  const VALID_TABS = ["my-subscriptions", "my-shops"];

  let activeTab = $state<string>("my-subscriptions");

  // Always set the tab (the valid selector, else the default): the Tabs
  // component applies its selection on a reactive write, and this also
  // re-syncs when the route's tab selector changes (e.g. a notification
  // navigates to /shops?tab=... while the page is already mounted).
  $effect(() => {
    const tab = query?.tab;
    activeTab = tab && VALID_TABS.includes(tab) ? tab : "my-subscriptions";
  });

  // Get all user's shops from the store (owned + subscribed)
  let myShops = $derived(
    stateValues(shops).filter((g) => g.owner === getUser()?.id),
  );
</script>

<div class="space-y-6">
  <Tabs style="underline" bind:selected={activeTab}>
    <TabItem key="my-subscriptions" title={m.shop_my_subscriptions()}>
      <div class="py-4">
        <Subscriptions />
      </div>
    </TabItem>

    <TabItem key="my-shops" title={m.shop_my_shops()} open={myShops.length > 0}>
      <div class="py-4 space-y-8">
        {#if myShops.length === 0}
          <div class="py-12 text-center">
            <p class="mb-4 text-text-1">
              {m.shop_none_owned()}
            </p>

            <Button onclick={() => createShop()}>
              {m.shop_create_first()}
            </Button>
          </div>
        {:else}
          <div>
            <ShopList shops={myShops} {users}>
              {#snippet sub(shop: Shop)}
                <ShopSubscriptions shopId={shop.id!} />
              {/snippet}
            </ShopList>
          </div>

          <Button onclick={() => createShop()}>
            {m.shop_create()}
          </Button>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
