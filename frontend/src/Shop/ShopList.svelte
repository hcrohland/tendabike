<script lang="ts">
  import { Button, Dropdown, DropdownItem } from "flowbite-svelte";
  import ShopCard from "./ShopCard.svelte";
  import { type Shop } from "../lib/shop";
  import { actions } from "../Widgets/Actions.svelte";
  import { user } from "../lib/store";
  import type { Snippet } from "svelte";
  import { DotsVerticalOutline } from "flowbite-svelte-icons";

  interface Props {
    shops: Shop[];
    sub?: Snippet<[Shop]>;
    showEnterShop?: boolean; // Only show "Enter Shop" if user has access
  }

  let { sub, shops }: Props = $props();
</script>

<div class="grid gap-4 grid-cols-1">
  {#each shops as shop}
    {@const isOwner = shop.owner === $user?.id}
    <ShopCard {shop} {isOwner} {sub}>
      {#if isOwner}
        <DotsVerticalOutline class="cursor-pointer" />
        <Dropdown>
          <DropdownItem onclick={() => $actions.editShop(shop)}>
            Edit Shop
          </DropdownItem>
          <DropdownItem onclick={() => $actions.deleteShop(shop)}>
            Delete Shop
          </DropdownItem>
        </Dropdown>
      {:else}
        <Button onclick={() => $actions.requestSubscription(shop)}>
          Request Subscription
        </Button>
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
