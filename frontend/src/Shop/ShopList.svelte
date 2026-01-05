<script lang="ts">
  import { DropdownItem } from "flowbite-svelte";
  import ShopCard from "./ShopCard.svelte";
  import type { Shop } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user, enterShop } from "../lib/store";
  import type { Snippet } from "svelte";

  interface Props {
    shops: Shop[];
    sub?: Snippet<[Shop]>;
    showEnterShop?: boolean; // Only show "Enter Shop" if user has access
  }

  let { sub, shops, showEnterShop = false }: Props = $props();
</script>

<div class="grid gap-4 grid-cols-1">
  {#each shops as shop}
    {@const isOwner = shop.owner === $user?.id}
    <ShopCard {shop} {isOwner} {sub}>
      {#if showEnterShop}
        <DropdownItem onclick={() => enterShop(shop.id!)}>
          Enter Shop
        </DropdownItem>
      {/if}
      {#if isOwner}
        <DropdownItem onclick={() => $actions.editShop(shop)}>
          Edit Shop
        </DropdownItem>
        <DropdownItem onclick={() => $actions.deleteShop(shop)}>
          Delete Shop
        </DropdownItem>
      {:else}
        <DropdownItem onclick={() => $actions.requestSubscription(shop)}>
          Request Subscription
        </DropdownItem>
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
