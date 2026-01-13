<script lang="ts">
  import { Table, TableHead, TableBody, Button, Input } from "flowbite-svelte";
  import { myfetch, handleError } from "../lib/store";
  import { user } from "../lib/user";
  import type { ShopSubscriptionFull } from "../lib/subscription";
  import { onMount } from "svelte";
  import { shops } from "../lib/shop";
  import SubscriptionRow from "./SubscriptionRow.svelte";

  interface Props {
    shopId: number;
  }

  let { shopId }: Props = $props();

  let subscriptions = $state<ShopSubscriptionFull[]>([]);
  let loading = $state(true);
  let respondingTo = $state<number | null>(null);
  let responseMessage = $state("");

  async function loadSubscriptions() {
    loading = true;
    try {
      subscriptions = await myfetch(`/api/shop/${shopId}/subscriptions`, "GET");
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

  function getUserName(userId: number): string {
    // For now, just show the user ID. In a real app, you'd fetch user names
    return $user?.id === userId ? "You" : `User #${userId}`;
  }

  onMount(loadSubscriptions);
</script>

<div class="space-y-6">
  {#if !($shops[shopId].auto_approve && subscriptions.length === 0)}
    <!-- Subscriptions list -->
    <div>
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          "Pending Subscription Requests"
        </h3>
        <Button size="sm" onclick={loadSubscriptions}>Refresh</Button>
      </div>

      {#if loading}
        <p class="text-gray-500 dark:text-gray-400">Loading...</p>
      {:else if subscriptions.length === 0}
        <p class="text-gray-500 dark:text-gray-400">
          "No pending subscription requests for this shop."
        </p>
      {:else}
        <div class="overflow-x-auto">
          <Table>
            <TableHead>
              <SubscriptionRow name={"User"} />
            </TableHead>
            <TableBody>
              {#each subscriptions as subscription}
                <SubscriptionRow
                  {subscription}
                  name={getUserName(subscription.user_id)}
                >
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
                  {:else}
                    <div class="flex gap-2">
                      {#if subscription.status === "pending"}
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={() => startResponding(subscription.id)}
                        >
                          Respond
                        </Button>
                      {/if}
                    </div>
                  {/if}
                </SubscriptionRow>
              {/each}
            </TableBody>
          </Table>
        </div>
      {/if}
    </div>
  {/if}
</div>
