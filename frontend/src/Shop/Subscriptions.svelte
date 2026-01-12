<script lang="ts">
  import {
    Table,
    TableHead,
    TableBody,
    Button,
    ButtonGroup,
    Dropdown,
    Checkbox,
    Li,
  } from "flowbite-svelte";
  import { onMount } from "svelte";
  import { myfetch, handleError } from "../lib/store";
  import { Shop, shops } from "../lib/shop";
  import ShopSearch from "./ShopSearch.svelte";
  import type { ShopSubscriptionFull } from "../lib/subscription";
  import SubscriptionRow from "./SubscriptionRow.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import { allGear, Part, parts } from "../lib/part";
  import { category } from "../lib/types";

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
    return (
      subscription.shop_name +
      ` by ` +
      subscription.shop_owner_firstname +
      ` ` +
      subscription.shop_owner_name
    );
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

  let mygear = $derived(allGear($parts, $category));

  async function registerGear(part: Part, shopid: number, checked: boolean) {
    try {
      if (checked) await Shop.registerPart(part, shopid);
      else await Shop.unregisterPart(part);
    } catch (error) {
      console.error("Error unregistering part:", error);
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
                      <ButtonGroup>
                        <Button size="xs" color="alternative">
                          Register {$category.name}s
                        </Button>
                        <Dropdown simple>
                          {#each mygear as gear}
                            <Li class="m-3">
                              <Checkbox
                                checked={gear.shop == subscription.shop_id}
                                onchange={(e: any) =>
                                  registerGear(
                                    gear,
                                    subscription.shop_id,
                                    e.target.checked,
                                  )}
                              >
                                {gear.name}
                              </Checkbox>
                            </Li>
                          {/each}
                        </Dropdown>
                        <Button
                          size="xs"
                          color="alternative"
                          onclick={() =>
                            startConfirmation(subscription.id, "unsubscribe")}
                          disabled={Object.values($parts).some(
                            (p) => p.shop == subscription.shop_id,
                          )}
                        >
                          Unsubscribe
                        </Button>
                      </ButtonGroup>
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
