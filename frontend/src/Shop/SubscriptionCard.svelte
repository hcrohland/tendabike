<script lang="ts">
  import Card from "../Widgets/Card.svelte";
  import Chip from "../Widgets/Chip.svelte";
  import Menu from "../Widgets/Menu.svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import type {
    ShopSubscription,
    SubscriptionStatus,
  } from "../lib/subscription";
  import { shops } from "../lib/shop.svelte";
  import { mainText } from "../Widgets/color.js";

  interface Props {
    subscription: ShopSubscription;
    menu?: Snippet;
    body?: Snippet;
  }

  let { subscription, menu, body }: Props = $props();

  let name = $derived(
    subscription.shop ? subscription.shop.name : shops[subscription.id!].name,
  );

  function statusLabel(status: SubscriptionStatus): string {
    switch (status) {
      case "active":
        return m.subscription_status_active();
      case "rejected":
        return m.subscription_status_rejected();
      case "pending":
        return m.subscription_status_pending();
      case "cancelled":
        return m.subscription_status_cancelled();
      default: {
        const _exhaustive: never = status;
        return _exhaustive;
      }
    }
  }
</script>

<Card color="secondary">
  <div class="flex items-center justify-between gap-1 sm:gap-2 p-1">
    <span class="font-medium text-sm min-w-0 truncate">{name}</span>
    <div class="flex items-center gap-1 sm:gap-2 shrink-0">
      <Chip
        value={statusLabel(subscription.status)}
        label=""
        color="secondary"
      />
      <span class={"text-xs " + mainText}>
        {new Date(subscription.created_at).toLocaleDateString()}
      </span>
      {#if menu}
        <Menu>
          {@render menu()}
        </Menu>
      {/if}
    </div>
  </div>
  {#if body}
    <div class="p-1 sm:p-2">
      {@render body()}
    </div>
  {/if}
</Card>
