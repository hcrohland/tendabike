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
  import * as m from "../../paraglide/messages";
  import { myfetch, handleError } from "../lib/store";
  import { Shop, shops } from "../lib/shop";
  import ShopSearch from "./ShopSearch.svelte";
  import type { ShopSubscription } from "../lib/subscription";
  import SubscriptionRow from "./SubscriptionRow.svelte";
  import { actions } from "../Widgets/Actions.svelte";
  import { allGear, Part, parts } from "../lib/part";
  import { category } from "../lib/types";

  interface Props {
    shopid?: number;
  }

  // coming from the subscription link
  let { shopid }: Props = $props();

  let subscriptions = $state<ShopSubscription[]>([]);
  let loading = $state(true);
  let confirmingAction = $state<{
    id: number;
    action: "unsubscribe" | "delete";
  } | null>(null);

  async function loadSubscriptions() {
    loading = true;
    try {
      subscriptions = await myfetch("/api/shop/subscriptions", "GET");
      // make sure subscribed shops are visible
      shops.updateMap(
        subscriptions.filter((s) => s.status == "active").map((s) => s.shop),
      );
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

  async function cancelSubscription(subscription: ShopSubscription) {
    try {
      await myfetch(
        `/api/shop/subscriptions/${subscription.id}`,
        "DELETE",
      ).then(() => shops.deleteItem(subscription.shop_id));
      confirmingAction = null;
      await loadSubscriptions();
    } catch (error) {
      handleError(error as Error);
    }
  }

  /// the registration link sets the shopid
  /// to trigger an automatic subscription dialog
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

  $effect(() => {
    loadSubscriptions();

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
  <div>
    <div class="mb-4 flex items-center justify-between">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
        {m.shop_my_subscriptions()}
      </h3>

      <Button size="sm" onclick={loadSubscriptions}>
        {m.header_refresh()}
      </Button>
    </div>

    {#if loading}
      <p class="text-text-1">{m.loading()}</p>
    {:else if subscriptions.length === 0}
      <p class="text-text-1">
        {m.shop_no_subscriptions()}
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
                {#if confirmingAction?.id === subscription.id}
                  <div class="flex flex-col gap-2 min-w-48">
                    <p class="text-sm text-gray-700 dark:text-gray-300">
                      {confirmingAction!.action === "unsubscribe"
                        ? m.shop_confirm_unsubscribe()
                        : m.shop_confirm_delete_request()}
                    </p>

                    <div class="flex gap-2">
                      <Button
                        size="xs"
                        color="red"
                        onclick={() => cancelSubscription(subscription)}
                      >
                        {m.action_confirm()}
                      </Button>

                      <Button
                        size="xs"
                        color="alternative"
                        onclick={cancelConfirmation}
                      >
                        {m.action_cancel()}
                      </Button>
                    </div>
                  </div>
                {:else}
                  <div class="flex gap-2">
                    {#if subscription.status === "pending"}
                      <Button
                        size="xs"
                        color="alternative"
                        onclick={() => cancelSubscription(subscription)}
                      >
                        {m.action_cancel()}
                      </Button>
                    {:else if subscription.status === "active"}
                      <ButtonGroup>
                        <Button size="xs" color="alternative">
                          {m.shop_register_gear({ category: $category.name })}
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
                            startConfirmation(subscription.id!, "unsubscribe")}
                          disabled={Object.values($parts).some(
                            (p) => p.shop == subscription.shop_id,
                          )}
                        >
                          {m.shop_unsubscribe()}
                        </Button>
                      </ButtonGroup>
                    {:else if subscription.status === "rejected"}
                      <Button
                        size="xs"
                        color="red"
                        onclick={() =>
                          startConfirmation(subscription.id!, "delete")}
                      >
                        {m.action_delete()}
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
