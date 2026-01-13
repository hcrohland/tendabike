<script lang="ts">
  import { Card, Badge } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import type { Shop } from "../lib/shop";
  import { onMount, onDestroy } from "svelte";
  import { users } from "../lib/user";

  interface Props {
    shop: Shop;
    isOwner?: boolean;
    sub?: Snippet<[Shop]>;
    children?: Snippet;
  }

  let { sub, shop, isOwner = false, children }: Props = $props();

  let owner = $derived($users[shop.owner]);

  // Get bikes registered to this shop
  let partsCount = $state(0);

  async function loadPartsCount() {
    if (shop.id && isOwner) {
      try {
        const parts = await shop.getParts();
        partsCount = parts.length;
      } catch (error) {
        console.error("Error loading shop parts:", error);
        // Silently fail - user might not have permission
      }
    }
  }

  function handleShopUpdate(event: CustomEvent) {
    if (event.detail.shopId === shop.id) {
      loadPartsCount();
    }
  }

  // Load parts count on mount
  onMount(() => {
    loadPartsCount();
    // Listen for shop updates
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
          by {owner.firstname}
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
          {partsCount === 1 ? "part" : "parts"}
        </Badge>
        <Badge color="green">Owner</Badge>
      {/if}
    </div>

    <div class="text-xs text-gray-500 dark:text-gray-400">
      Created {new Date(shop.created_at).toLocaleDateString()}
    </div>
    {#if sub}
      {@render sub(shop)}
    {/if}
  </div>
</Card>
