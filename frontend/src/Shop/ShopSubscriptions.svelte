<script lang="ts">
  import { Table, TableHead, TableBody, Button, Input } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
  import { myfetch, handleError } from "../lib/store";
  import type { ShopSubscription } from "../lib/subscription";
  import { onMount } from "svelte";
  import { shops } from "../lib/shop";
  import SubscriptionRow from "./SubscriptionRow.svelte";

  interface Props {
    shopId: number;
  }

  let { shopId }: Props = $props();

  let subscriptions = $state<ShopSubscription[]>([]);
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

  onMount(loadSubscriptions);
</script>

<div class="space-y-6">
  {#if !($shops[shopId].auto_approve && subscriptions.length === 0)}
    <div>
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
          {m.shop_pending_requests()}
        </h3>

        <Button size="sm" onclick={loadSubscriptions}>
          {m.header_refresh()}
        </Button>
      </div>

      {#if loading}
        <p class="text-text-1">{m.loading()}</p>
      {:else if subscriptions.length === 0}
        <p class="text-text-1">
          {m.shop_no_pending_requests()}
        </p>
      {:else}
        <div class="overflow-x-auto">
          <Table>
            <TableHead>
              <SubscriptionRow />
            </TableHead>
            <TableBody>
              {#each subscriptions as subscription}
                <SubscriptionRow {subscription}>
                  {#if respondingTo === subscription.id}
                    <div class="flex min-w-48 flex-col gap-2">
                      <Input
                        type="text"
                        bind:value={responseMessage}
                        placeholder={m.shop_response_placeholder()}
                        size="sm"
                      />
                      <div class="flex gap-2">
                        <Button
                          size="xs"
                          color="green"
                          onclick={() =>
                            approveSubscription(
                              subscription.id!,
                              responseMessage,
                            )}
                        >
                          {m.action_approve()}
                        </Button>
                        <Button
                          size="xs"
                          color="red"
                          onclick={() =>
                            rejectSubscription(
                              subscription.id!,
                              responseMessage,
                            )}
                        >
                          {m.action_reject()}
                        </Button>
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={cancelResponding}
                        >
                          {m.action_cancel()}
                        </Button>
                      </div>
                    </div>
                  {:else if subscription.status === "pending"}
                    <div class="flex gap-2">
                      <Button
                        size="xs"
                        color="alternative"
                        onclick={() => startResponding(subscription.id!)}
                      >
                        {m.action_respond()}
                      </Button>
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
