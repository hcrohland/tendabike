<script lang="ts">
  import { Table, TableHead, TableBody, Button } from "flowbite-svelte";
  import { onMount } from "svelte";
  import { myfetch, handleError } from "../lib/store";
  import { Shop, shops } from "../lib/shop";
  import ShopSearch from "./ShopSearch.svelte";
  import type { ShopSubscriptionFull } from "../lib/subscription";
  import SubscriptionRow from "./SubscriptionRow.svelte";
  import { actions } from "../Widgets/Actions.svelte";

  interface Props {
    shopid?: number;
  }

  let { shopid }: Props = $props();

  let subscriptions = $state<ShopSubscriptionFull[]>([]);
  let loading = $state(true);
  let confirmingAction = $state<{
    id: number;
    action: "unsubscribe" | "delete";
  } | null>(null);

  async function loadSubscriptions() {
    loading = true;
    try {
      subscriptions = await myfetch("/api/shop/subscriptions", "GET");
    } catch (error) {
      handleError(error as Error);
    } finally {
      loading = false;
    }
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

  function getShopName(subscription: ShopSubscriptionFull): string {
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

  async function register() {
    if (shopid) {
      myfetch(`/api/shop/` + shopid)
        .then((s) => new Shop(s))
        .then((shop) => $actions.requestSubscription(shop))
        .catch(handleError);
      shopid = undefined;
    }
  }

  onMount(() => {
    loadSubscriptions();

    // Listen for subscription updates from other components
    const handleSubscriptionUpdate = () => {
      loadSubscriptions();
    };
    window.addEventListener("subscription-updated", handleSubscriptionUpdate);

    register();

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
        My Subscriptions
      </h3>
      <Button size="sm" onclick={loadSubscriptions}>Refresh</Button>
    </div>

    {#if loading}
      <p class="text-gray-500 dark:text-gray-400">Loading...</p>
    {:else if subscriptions.length === 0}
      <p class="text-gray-500 dark:text-gray-400">
        You haven't subscribed to any shops yet.
      </p>
    {:else}
      <div class="overflow-x-auto">
        <Table>
          <TableHead>
            <SubscriptionRow name={"Shop"} />
          </TableHead>
          <TableBody>
            {#each subscriptions as subscription}
              <SubscriptionRow {subscription} name={getShopName(subscription)}>
                {#if confirmingAction?.id === subscription.id}
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
                      <Button
                        size="xs"
                        color="alternative"
                        onclick={() => cancelSubscription(subscription.id)}
                      >
                        Cancel
                      </Button>
                    {:else if subscription.status === "active"}
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
              </SubscriptionRow>
            {/each}
          </TableBody>
        </Table>
      </div>
    {/if}
  </div>
</div>

<ShopSearch {subscriptions} />
