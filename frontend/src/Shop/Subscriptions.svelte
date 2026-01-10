<script lang="ts">
  import {
    Table,
    TableHead,
    TableHeadCell,
    TableBody,
    TableBodyRow,
    TableBodyCell,
    Badge,
    Button,
    Input,
    Label,
    Spinner,
    Tooltip,
  } from "flowbite-svelte";
  import { onMount } from "svelte";
  import { myfetch, handleError, enterShop } from "../lib/store";
  import { shops, Shop } from "../lib/shop";
  import { user } from "../lib/store";
  import ShopList from "./ShopList.svelte";

  interface ShopSubscription {
    id: number;
    shop_id: number;
    shop_name?: string;
    shop_owner_firstname?: string;
    shop_owner_name?: string;
    user_id: number;
    status: "pending" | "active" | "rejected" | "cancelled";
    message?: string;
    response_message?: string;
    created_at: string;
    updated_at: string;
  }

  interface Props {
    shopId?: number;
    showMySubscriptions?: boolean;
  }

  let { shopId, showMySubscriptions = false }: Props = $props();

  let subscriptions = $state<ShopSubscription[]>([]);
  let loading = $state(true);
  let searchQuery = $state("");
  let searchResults = $state<Shop[]>([]);
  let isSearching = $state(false);
  let respondingTo = $state<number | null>(null);
  let responseMessage = $state("");
  let confirmingAction = $state<{
    id: number;
    action: "unsubscribe" | "delete";
  } | null>(null);

  async function loadSubscriptions() {
    loading = true;
    try {
      const url = showMySubscriptions
        ? "/api/shop/subscriptions"
        : `/api/shop/${shopId}/subscriptions`;
      subscriptions = await myfetch(url, "GET");
    } catch (error) {
      handleError(error as Error);
    } finally {
      loading = false;
    }
  }

  async function approveSubscription(subscriptionId: number, message?: string) {
    try {
      await myfetch(
        `/api/shop/subscriptions/${subscriptionId}/approve`,
        "POST",
        {
          message: message || null,
        },
      );
      respondingTo = null;
      responseMessage = "";
      await loadSubscriptions();
    } catch (error) {
      handleError(error as Error);
    }
  }

  async function rejectSubscription(subscriptionId: number, message?: string) {
    try {
      await myfetch(
        `/api/shop/subscriptions/${subscriptionId}/reject`,
        "POST",
        {
          message: message || null,
        },
      );
      respondingTo = null;
      responseMessage = "";
      await loadSubscriptions();
    } catch (error) {
      handleError(error as Error);
    }
  }

  function startResponding(subscriptionId: number) {
    respondingTo = subscriptionId;
    responseMessage = "";
  }

  function cancelResponding() {
    respondingTo = null;
    responseMessage = "";
  }

  function startConfirmation(
    subscriptionId: number,
    action: "unsubscribe" | "delete",
  ) {
    confirmingAction = { id: subscriptionId, action };
  }

  function cancelConfirmation() {
    confirmingAction = null;
  }

  async function cancelSubscription(subscriptionId: number) {
    try {
      await myfetch(`/api/shop/subscriptions/${subscriptionId}`, "DELETE").then(
        () => shops.deleteItem(subscriptionId),
      );
      confirmingAction = null;
      await loadSubscriptions();
    } catch (error) {
      handleError(error as Error);
    }
  }

  function getShopName(subscription: ShopSubscription): string {
    // If subscription includes shop details, use them
    if (subscription.shop_name) {
      const ownerName =
        subscription.shop_owner_firstname && subscription.shop_owner_name
          ? ` by ${subscription.shop_owner_firstname} ${subscription.shop_owner_name}`
          : "";
      return `${subscription.shop_name}${ownerName}`;
    }

    // Fall back to looking up in shops store
    const shop = Object.values($shops).find(
      (g) => g.id === subscription.shop_id,
    );
    if (!shop) return `Shop #${subscription.shop_id}`;

    const ownerName =
      shop.owner_firstname && shop.owner_name
        ? ` (${shop.owner_firstname} ${shop.owner_name})`
        : "";
    return `${shop.name}${ownerName}`;
  }

  function getUserName(userId: number): string {
    // For now, just show the user ID. In a real app, you'd fetch user names
    return $user?.id === userId ? "You" : `User #${userId}`;
  }

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

  async function performSearch() {
    if (!searchQuery.trim()) {
      searchResults = [];
      return;
    }

    isSearching = true;
    try {
      const results = await myfetch(
        `/api/shop/search?q=${encodeURIComponent(searchQuery)}`,
        "GET",
      );
      searchResults = results
        .map((g: any) => new Shop(g))
        .filter(
          (s: any) =>
            s.owner != $user!.id &&
            subscriptions.every((su) => su.shop_id != s.id),
        );
    } catch (error) {
      handleError(error as Error);
    } finally {
      isSearching = false;
    }
  }

  function handleSearchInput() {
    // Debounce search
    const timeoutId = setTimeout(() => {
      performSearch();
    }, 300);
    return () => clearTimeout(timeoutId);
  }

  onMount(() => {
    loadSubscriptions();

    // Listen for subscription updates from other components
    const handleSubscriptionUpdate = () => {
      if (showMySubscriptions) {
        loadSubscriptions();
      }
    };
    window.addEventListener("subscription-updated", handleSubscriptionUpdate);

    return () => {
      window.removeEventListener(
        "subscription-updated",
        handleSubscriptionUpdate,
      );
    };
  });
</script>

<div class="space-y-6">
  <!-- Subscriptions list -->
  <div>
    <div class="mb-4 flex items-center justify-between">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
        {showMySubscriptions
          ? "My Subscriptions"
          : "Pending Subscription Requests"}
      </h3>
      <Button size="sm" onclick={loadSubscriptions}>Refresh</Button>
    </div>

    {#if loading}
      <p class="text-gray-500 dark:text-gray-400">Loading...</p>
    {:else if subscriptions.length === 0}
      <p class="text-gray-500 dark:text-gray-400">
        {showMySubscriptions
          ? "You haven't subscribed to any shops yet."
          : "No pending subscription requests for this shop."}
      </p>
    {:else}
      <div class="overflow-x-auto">
        <Table>
          <TableHead>
            {#if showMySubscriptions}
              <TableHeadCell>Shop</TableHeadCell>
            {:else}
              <TableHeadCell>User</TableHeadCell>
            {/if}
            <TableHeadCell>Request Message</TableHeadCell>
            <TableHeadCell>Response</TableHeadCell>
            <TableHeadCell>Status</TableHeadCell>
            <TableHeadCell>Date</TableHeadCell>
            <TableHeadCell>Actions</TableHeadCell>
          </TableHead>
          <TableBody>
            {#each subscriptions as subscription}
              <TableBodyRow>
                {#if showMySubscriptions}
                  <TableBodyCell>{getShopName(subscription)}</TableBodyCell>
                {:else}
                  <TableBodyCell>
                    {getUserName(subscription.user_id)}
                  </TableBodyCell>
                {/if}
                <TableBodyCell>
                  {#if subscription.message && subscription.message.length > 50}
                    <span
                      id="request-msg-{subscription.id}"
                      class="text-sm text-gray-600 dark:text-gray-400 cursor-help"
                    >
                      {truncateText(subscription.message)}
                    </span>
                    <Tooltip
                      triggeredBy="#request-msg-{subscription.id}"
                      class="max-w-xs"
                    >
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
                    <Tooltip
                      triggeredBy="#response-msg-{subscription.id}"
                      class="max-w-xs"
                    >
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
                  {#if respondingTo === subscription.id}
                    <!-- Response form -->
                    <div class="flex flex-col gap-2 min-w-48">
                      <Input
                        type="text"
                        bind:value={responseMessage}
                        placeholder="Optional message..."
                        size="sm"
                      />
                      <div class="flex gap-2">
                        <Button
                          size="xs"
                          color="green"
                          onclick={() =>
                            approveSubscription(
                              subscription.id,
                              responseMessage,
                            )}
                        >
                          Approve
                        </Button>
                        <Button
                          size="xs"
                          color="red"
                          onclick={() =>
                            rejectSubscription(
                              subscription.id,
                              responseMessage,
                            )}
                        >
                          Reject
                        </Button>
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={cancelResponding}
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  {:else if confirmingAction?.id === subscription.id}
                    <!-- Confirmation dialog -->
                    <div class="flex flex-col gap-2 min-w-48">
                      <p class="text-sm text-gray-700 dark:text-gray-300">
                        {confirmingAction.action === "unsubscribe"
                          ? "Unsubscribe from this shop?"
                          : "Delete this rejected request?"}
                      </p>
                      <div class="flex gap-2">
                        <Button
                          size="xs"
                          color="red"
                          onclick={() => cancelSubscription(subscription.id)}
                        >
                          Confirm
                        </Button>
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={cancelConfirmation}
                        >
                          Cancel
                        </Button>
                      </div>
                    </div>
                  {:else}
                    <div class="flex gap-2">
                      {#if subscription.status === "pending"}
                        {#if showMySubscriptions}
                          <Button
                            size="xs"
                            color="alternative"
                            onclick={() => cancelSubscription(subscription.id)}
                          >
                            Cancel
                          </Button>
                        {:else}
                          <Button
                            size="xs"
                            color="alternative"
                            onclick={() => startResponding(subscription.id)}
                          >
                            Respond
                          </Button>
                        {/if}
                      {:else if subscription.status === "active" && showMySubscriptions}
                        <!-- <Button
                          size="xs"
                          color="blue"
                          onclick={() => enterShop(subscription.shop_id)}
                        >
                          Enter Shop
                        </Button> -->
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={() =>
                            startConfirmation(subscription.id, "unsubscribe")}
                        >
                          Unsubscribe
                        </Button>
                      {:else if subscription.status === "rejected"}
                        <Button
                          size="xs"
                          color="red"
                          onclick={() =>
                            startConfirmation(subscription.id, "delete")}
                        >
                          Delete
                        </Button>
                      {/if}
                    </div>
                  {/if}
                </TableBodyCell>
              </TableBodyRow>
            {/each}
          </TableBody>
        </Table>
      </div>
    {/if}
  </div>

  {#if showMySubscriptions}
    <!-- Divider -->
    <hr class="border-gray-200 dark:border-gray-700" />

    <!-- Search for shops -->
    <div>
      <h3 class="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
        Find Shops
      </h3>
      <div class="space-y-4">
        <div>
          <div class="flex gap-2">
            <Input
              id="search"
              type="text"
              bind:value={searchQuery}
              oninput={handleSearchInput}
              placeholder="Search for shops to request subscription"
            />
          </div>
        </div>

        {#if isSearching}
          <div class="flex justify-center py-8">
            <Spinner />
          </div>
        {:else if searchResults.length > 0}
          <ShopList shops={searchResults} />
        {:else if searchQuery.trim()}
          <p class="py-8 text-center text-gray-500 dark:text-gray-400">
            No shops found matching "{searchQuery}"
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>
