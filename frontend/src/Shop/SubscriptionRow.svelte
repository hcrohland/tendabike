<script lang="ts">
  import {
    Badge,
    TableBodyCell,
    TableBodyRow,
    TableHeadCell,
  } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import type { ShopSubscription } from "../lib/subscription";
  import { shops } from "../lib/shop";

  interface Props {
    subscription?: ShopSubscription;
    children?: Snippet;
  }

  let { subscription, children }: Props = $props();

  let name = $derived(
    subscription
      ? subscription.shop
        ? subscription.shop.name
        : shops[subscription.id!].name
      : "",
  );

  function getStatusColor(
    status: string,
  ): "green" | "red" | "yellow" | "blue" | "gray" {
    switch (status) {
      case "active":
        return "green";
      case "rejected":
        return "red";
      case "pending":
        return "yellow";
      case "cancelled":
        return "gray";
      default:
        return "blue";
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case "active":
        return m.subscription_status_active();
      case "rejected":
        return m.subscription_status_rejected();
      case "pending":
        return m.subscription_status_pending();
      case "cancelled":
        return m.subscription_status_cancelled();
      default:
        return status;
    }
  }
</script>

{#if subscription}
  <TableBodyRow>
    <TableBodyCell>{name}</TableBodyCell>

    <TableBodyCell>
      <Badge color={getStatusColor(subscription.status)}>
        {statusLabel(subscription.status)}
      </Badge>
    </TableBodyCell>

    <TableBodyCell>
      <span class="text-sm text-gray-600 dark:text-gray-400">
        {new Date(subscription.created_at).toLocaleDateString()}
      </span>
    </TableBodyCell>

    <TableBodyCell>
      {#if children}
        {@render children()}
      {/if}
    </TableBodyCell>
  </TableBodyRow>
{:else}
  <TableHeadCell>{m.shop()}</TableHeadCell>
  <TableHeadCell>{m.subscription_status()}</TableHeadCell>
  <TableHeadCell>{m.subscription_date()}</TableHeadCell>
  <TableHeadCell>{m.subscription_actions()}</TableHeadCell>
{/if}
