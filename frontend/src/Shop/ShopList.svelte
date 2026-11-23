<script lang="ts">
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";

  import ShopCard from "./ShopCard.svelte";
  import { type Shop } from "../lib/shop.svelte";
  import { requestSubscription } from "./SubscriptionRequestModal.svelte";
  import {
    getUser,
    users as global_users,
    type UserPublic,
  } from "../lib/user.svelte";
  import ShopOwnerMenu from "./ShopOwnerMenu.svelte";
  import { type Map } from "../lib/mapable.svelte";

  interface Props {
    shops: Shop[];
    users: Map<UserPublic>;
    sub?: Snippet<[Shop]>;
  }

  let { sub, shops, users }: Props = $props();

  function request(shop: Shop) {
    // add the owner to the global stores
    global_users.updateMap([users[shop.owner]]);
    requestSubscription(shop);
  }
</script>

<div class="flex flex-col gap-2 sm:gap-3">
  {#each shops as shop}
    {@const isOwner = shop.owner === getUser()?.id}
    <ShopCard {shop} {isOwner} {users} {sub}>
      {#if isOwner}
        <ShopOwnerMenu {shop} />
      {:else}
        <button
          onclick={() => request(shop)}
          class="shrink-0 cursor-pointer rounded-lg border border-border-subtle bg-surface-1 px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-primary sm:px-3 sm:py-1.5"
        >
          {m.shop_request_subscription()}
        </button>
      {/if}
    </ShopCard>
  {/each}
</div>

{#if shops.length === 0}
  <div class="py-12 text-center">
    <p class="text-text-1">
      {m.shop_none_found()}
    </p>
  </div>
{/if}
