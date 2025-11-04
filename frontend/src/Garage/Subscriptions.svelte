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
  import { myfetch, handleError, enterGarage } from "../lib/store";
  import { garages, Garage } from "../lib/garage";
  import { user } from "../lib/store";
  import GarageList from "./GarageList.svelte";

  interface GarageSubscription {
    id: number;
    garage_id: number;
    garage_name?: string;
    garage_owner_firstname?: string;
    garage_owner_name?: string;
    user_id: number;
    status: "pending" | "active" | "rejected" | "cancelled";
    message?: string;
    response_message?: string;
    created_at: string;
    updated_at: string;
  }

  interface Props {
    garageId?: number;
    showMySubscriptions?: boolean;
  }

  let { garageId, showMySubscriptions = false }: Props = $props();

  let subscriptions = $state<GarageSubscription[]>([]);
  let loading = $state(true);
  let searchQuery = $state("");
  let searchResults = $state<Garage[]>([]);
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
        ? "/api/garage/subscriptions"
        : `/api/garage/${garageId}/subscriptions`;
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
        `/api/garage/subscriptions/${subscriptionId}/approve`,
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
        `/api/garage/subscriptions/${subscriptionId}/reject`,
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
      await myfetch(`/api/garage/subscriptions/${subscriptionId}`, "DELETE");
      confirmingAction = null;
      await loadSubscriptions();
    } catch (error) {
      handleError(error as Error);
    }
  }

  function getGarageName(subscription: GarageSubscription): string {
    // If subscription includes garage details, use them
    if (subscription.garage_name) {
      const ownerName =
        subscription.garage_owner_firstname && subscription.garage_owner_name
          ? ` (${subscription.garage_owner_firstname} ${subscription.garage_owner_name})`
          : "";
      return `${subscription.garage_name}${ownerName}`;
    }

    // Fall back to looking up in garages store
    const garage = Object.values($garages).find(
      (g) => g.id === subscription.garage_id,
    );
    if (!garage) return `Garage #${subscription.garage_id}`;

    const ownerName =
      garage.owner_firstname && garage.owner_name
        ? ` (${garage.owner_firstname} ${garage.owner_name})`
        : "";
    return `${garage.name}${ownerName}`;
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
        `/api/garage/search?q=${encodeURIComponent(searchQuery)}`,
        "GET",
      );
      searchResults = results.map((g: any) => new Garage(g));
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
          ? "You haven't subscribed to any garages yet."
          : "No pending subscription requests for this garage."}
      </p>
    {:else}
      <div class="overflow-x-auto">
        <Table>
          <TableHead>
            {#if showMySubscriptions}
              <TableHeadCell>Garage</TableHeadCell>
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
                  <TableBodyCell>{getGarageName(subscription)}</TableBodyCell>
                {:else}
                  <TableBodyCell
                    >{getUserName(subscription.user_id)}</TableBodyCell
                  >
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
                          ? "Unsubscribe from this garage?"
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
                        <Button
                          size="xs"
                          color="blue"
                          onclick={() => enterGarage(subscription.garage_id)}
                        >
                          Enter Garage
                        </Button>
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

    <!-- Search for garages -->
    <div>
      <h3 class="mb-4 text-lg font-semibold text-gray-900 dark:text-white">
        Find Garages
      </h3>
      <div class="space-y-4">
        <div>
          <Label for="search" class="mb-2">Search for garages</Label>
          <div class="flex gap-2">
            <Input
              id="search"
              type="text"
              bind:value={searchQuery}
              oninput={handleSearchInput}
              placeholder="Search for garages to request subscription"
            />
            <Button onclick={performSearch}>Search</Button>
          </div>
        </div>

        {#if isSearching}
          <div class="flex justify-center py-8">
            <Spinner />
          </div>
        {:else if searchResults.length > 0}
          <GarageList garages={searchResults} />
        {:else if searchQuery.trim()}
          <p class="py-8 text-center text-gray-500 dark:text-gray-400">
            No garages found matching "{searchQuery}"
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>
