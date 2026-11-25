<script lang="ts">
  import { Button, Input } from "flowbite-svelte";
  import { DropdownItem } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
  import { myfetch, handleError } from "../lib/utils";
  import type { ShopSubscription } from "../lib/subscription";
  import { shops } from "../lib/shop.svelte";
  import SubscriptionCard from "./SubscriptionCard.svelte";

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

  $effect(() => {
    loadSubscriptions();
  });
</script>

<div class="space-y-6">
  {#if !(shops[shopId].auto_approve && subscriptions.length === 0)}
    <div>
      <div class="mb-4 flex items-center justify-between">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-300">
          {m.shop_pending_requests()}
        </h3>

        <Button
          size="sm"
          color="light"
          class="bg-secondary-300! dark:bg-secondary-600!"
          onclick={loadSubscriptions}
        >
          {m.header_refresh()}
        </Button>
      </div>

      {#if loading}
        <p class="text-gray-600 dark:text-gray-400">{m.loading()}</p>
      {:else if subscriptions.length === 0}
        <p class="text-gray-600 dark:text-gray-400">
          {m.shop_no_pending_requests()}
        </p>
      {:else}
        <div class="flex flex-col gap-1 sm:gap-2">
          {#each subscriptions as subscription (subscription.id)}
            {#snippet menu()}
              {#if subscription.status === "pending"}
                <DropdownItem onclick={() => startResponding(subscription.id!)}>
                  {m.action_respond()}
                </DropdownItem>
              {/if}
            {/snippet}

            {#snippet body()}
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
                      approveSubscription(subscription.id!, responseMessage)}
                  >
                    {m.action_approve()}
                  </Button>
                  <Button
                    size="xs"
                    color="red"
                    onclick={() =>
                      rejectSubscription(subscription.id!, responseMessage)}
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
            {/snippet}

            <SubscriptionCard
              {subscription}
              menu={subscription.status === "pending" ? menu : undefined}
              body={respondingTo === subscription.id ? body : undefined}
            />
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
