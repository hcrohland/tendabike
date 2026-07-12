<script lang="ts">
  import { Card, Badge } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import { onMount, onDestroy } from "svelte";
  import * as m from "../../paraglide/messages";

  import type { Shop } from "../lib/shop";
  import { type UserPublic } from "../lib/user";
  import { type Map } from "../lib/mapable";

  interface Props {
    shop: Shop;
    users: Map<UserPublic>;
    isOwner?: boolean;
    sub?: Snippet<[Shop]>;
    children?: Snippet;
  }

  let { sub, shop, users, isOwner = false, children }: Props = $props();

  let owner = $derived(users[shop.owner]);

  let partsCount = $state(0);

  async function loadPartsCount() {
    if (shop.id && isOwner) {
      try {
        const parts = await shop.getParts();
        partsCount = parts.length;
      } catch (error) {
        console.error("Error loading shop parts:", error);
      }
    }
  }

  function handleShopUpdate(event: CustomEvent) {
    if (event.detail.shopId === shop.id) {
      loadPartsCount();
    }
  }

  onMount(() => {
    loadPartsCount();
    window.addEventListener("shop-updated", handleShopUpdate as EventListener);
  });

  onDestroy(() => {
    window.removeEventListener(
      "shop-updated",
      handleShopUpdate as EventListener,
    );
  });
</script>

<Card size="xl" class="relative col-auto p-4">
  {#if children}
    <div class="absolute top-4 right-4">
      {@render children?.()}
    </div>
  {/if}

  <div class="space-y-3">
    <div>
      <h5
        class="text-xl font-bold tracking-tight text-gray-900 dark:text-white"
      >
        {shop.name}
      </h5>

      {#if !isOwner && owner.firstname && owner.name}
        <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
          {m.shop_by()}
          {owner.firstname}
          {owner.name}
        </p>
      {/if}

      {#if shop.description}
        <p class="mt-2 text-sm text-gray-600 dark:text-gray-400">
          {shop.description}
        </p>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if isOwner}
        <Badge color="blue">
          {partsCount}
          {partsCount === 1 ? m.shop_part() : m.shop_parts()}
        </Badge>

        <Badge color="green">
          {m.shop_owner()}
        </Badge>
      {/if}
    </div>

    <div class="text-xs text-text-1">
      {m.shop_created()}
      {new Date(shop.created_at).toLocaleDateString()}
    </div>

    {#if sub}
      {@render sub(shop)}
    {/if}
  </div>
</Card>
