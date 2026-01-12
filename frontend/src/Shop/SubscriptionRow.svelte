<script lang="ts">
  import {
    Badge,
    TableBodyCell,
    TableBodyRow,
    TableHeadCell,
    Tooltip,
  } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import type { ShopSubscriptionFull } from "../lib/subscription";

  interface Props {
    subscription?: ShopSubscriptionFull;
    name: string;
    children?: Snippet;
  }

  let { subscription, name, children }: Props = $props();

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

  function truncateText(
    text: string | undefined,
    maxLength: number = 50,
  ): string {
    if (!text) return "-";
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength) + "...";
  }
</script>

{#if subscription}
  <TableBodyRow>
    <TableBodyCell>{name}</TableBodyCell>
    <TableBodyCell>
      {#if subscription.message && subscription.message.length > 50}
        <span
          id="request-msg-{subscription.id}"
          class="text-sm text-gray-600 dark:text-gray-400 cursor-help"
        >
          {truncateText(subscription.message)}
        </span>
        <Tooltip triggeredBy="#request-msg-{subscription.id}" class="max-w-xs">
          {subscription.message}
        </Tooltip>
      {:else}
        <span class="text-sm text-gray-600 dark:text-gray-400">
          {subscription.message || "-"}
        </span>
      {/if}
    </TableBodyCell>
    <TableBodyCell>
      {#if subscription.response_message && subscription.response_message.length > 50}
        <span
          id="response-msg-{subscription.id}"
          class="text-sm text-gray-600 dark:text-gray-400 cursor-help"
        >
          {truncateText(subscription.response_message)}
        </span>
        <Tooltip triggeredBy="#response-msg-{subscription.id}" class="max-w-xs">
          {subscription.response_message}
        </Tooltip>
      {:else}
        <span class="text-sm text-gray-600 dark:text-gray-400">
          {subscription.response_message || "-"}
        </span>
      {/if}
    </TableBodyCell>
    <TableBodyCell>
      <Badge color={getStatusColor(subscription.status)}>
        {subscription.status}
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
  <!-- Header Row -->
  <TableHeadCell>{name}</TableHeadCell>
  <TableHeadCell>Request Message</TableHeadCell>
  <TableHeadCell>Response</TableHeadCell>
  <TableHeadCell>Status</TableHeadCell>
  <TableHeadCell>Date</TableHeadCell>
  <TableHeadCell>Actions</TableHeadCell>
{/if}
