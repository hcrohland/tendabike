<script lang="ts">
  import { Button } from "flowbite-svelte";
  import ShopCard from "./ShopCard.svelte";
  import { type Shop } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user, users as global_users, type UserPublic } from "../lib/user";
  import type { Snippet } from "svelte";
  import ShopOwnerMenu from "./ShopOwnerMenu.svelte";
  import { type Map } from "../lib/mapable";

  interface Props {
    shops: Shop[];
    users: Map<UserPublic>;
    sub?: Snippet<[Shop]>;
  }

  let { sub, shops, users }: Props = $props();

  function request(shop: Shop) {
    // add the owner to the global stores
    global_users.updateMap([users[shop.owner]]);
    $actions.requestSubscription(shop);
  }
</script>

<div class="grid gap-4 grid-cols-1">
  {#each shops as shop}
    {@const isOwner = shop.owner === $user?.id}
    <ShopCard {shop} {isOwner} {users} {sub}>
      {#if isOwner}
        <ShopOwnerMenu {shop} />
      {:else}
        <Button onclick={() => request(shop)}>Request Subscription</Button>
      {/if}
    </ShopCard>
  {/each}
</div>

{#if shops.length === 0}
  <div class="py-12 text-center">
    <p class="text-gray-500 dark:text-gray-400">
      No shops found. Create your first shop to get started!
    </p>
  </div>
{/if}
