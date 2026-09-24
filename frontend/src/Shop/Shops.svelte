<script lang="ts">
  import { Button, Tabs, TabItem } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";

  import ShopList from "./ShopList.svelte";
  import Subscriptions from "./Subscriptions.svelte";
  import { Shop, shops } from "../lib/shop";
  import { getActions } from "../Widgets/Actions.svelte";
  import { getUser, users } from "../lib/user";
  import { stateValues } from "../lib/mapable.svelte";
  import ShopSubscriptions from "./ShopSubscriptions.svelte";

  let activeTab = $state<string>("my-subscriptions");

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

            <Button onclick={() => getActions()!.createShop()}>
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

          <Button onclick={() => getActions()!.createShop()}>
            {m.shop_create()}
          </Button>
        {/if}
      </div>
    </TabItem>
  </Tabs>
</div>
