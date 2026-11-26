<script lang="ts">
  import Card from "../Widgets/Card.svelte";
  import Chip from "../Widgets/Chip.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";

  import type { Shop } from "../lib/shop.svelte";
  import { type UserPublic } from "../lib/user.svelte";
  import { type Map } from "../lib/mapable.svelte";
  import { mainText } from "../Widgets/color.js";

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

  $effect(() => {
    loadPartsCount();
    window.addEventListener("shop-updated", handleShopUpdate as EventListener);
    return () => {
      window.removeEventListener(
        "shop-updated",
        handleShopUpdate as EventListener,
      );
    };
  });
</script>

<Card color="secondary">
  <!-- Header: name + per-entity action (owner menu, non-owner request button) -->
  <div class="flex items-center justify-between gap-1 sm:gap-2">
    <span class="font-bold text-base sm:text-lg wrap-break-word min-w-0">
      {shop.name}
    </span>
    {#if children}
      {#if isOwner}
        <Menu>
          {@render children?.()}
        </Menu>
      {:else}
        {@render children?.()}
      {/if}
    {/if}
  </div>

  {#if !isOwner && owner.firstname && owner.name}
    <div class={"mt-1 text-sm " + mainText}>
      {m.shop_by()}
      {owner.firstname}
      {owner.name}
    </div>
  {/if}

  {#if shop.description}
    <div class={"mt-1 text-sm " + mainText}>
      {shop.description}
    </div>
  {/if}

  {#if isOwner}
    <div class="mt-2 flex items-center gap-1 sm:gap-2">
      <Chip
        value={String(partsCount)}
        label={partsCount === 1 ? m.shop_part() : m.shop_parts()}
        color="secondary"
      />
      <Chip value={m.shop_owner()} label="" color="secondary" />
    </div>
  {/if}

  <div class={"mt-2 text-xs " + mainText}>
    {m.shop_created()}
    {new Date(shop.created_at).toLocaleDateString()}
  </div>

  {#if sub}
    <div class="mt-2">
      {@render sub(shop)}
    </div>
  {/if}
</Card>
