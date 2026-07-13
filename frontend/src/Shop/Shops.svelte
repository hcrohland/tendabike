<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";

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

            <Button onclick={() => $actions.createShop()}>
              {m.shop_create_first()}
            </Button>
          </div>
        {:else}
          <div>
            <ShopList shops={myShops} users={$users}>
              {#snippet sub(shop: Shop)}
                <ShopSubscriptions shopId={shop.id!} />
              {/snippet}
            </ShopList>
          </div>

          <Button onclick={() => $actions.createShop()}>
            {m.shop_create()}
          </Button>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
