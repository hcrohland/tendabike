<script lang="ts">
  import { Button, ButtonGroup, Dropdown, Checkbox, Li } from "flowbite-svelte";
  import { DropdownItem } from "flowbite-svelte";
  import * as m from "../../paraglide/messages";
  import { myfetch, handleError } from "../lib/utils";
  import { Shop, shops } from "../lib/shop.svelte";
  import ShopSearch from "./ShopSearch.svelte";
  import type { ShopSubscription } from "../lib/subscription";
  import { subscriptions as subscriptionsMap } from "../lib/subscription";
  import SubscriptionCard from "./SubscriptionCard.svelte";
  import { requestSubscription } from "./SubscriptionRequestModal.svelte";
  import { allGear, Part, parts } from "../lib/part";
  import { stateValues } from "../lib/mapable.svelte";
  import { getCategory } from "../lib/types.svelte";

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
      subscriptionsMap.setMap(subscriptions.map((s) => s));
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
        .then((shop) => requestSubscription(shop))
        .catch(handleError);
      shopid = undefined;
    }
  }

  let mygear = $derived(allGear(getCategory()!));

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
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-300">
        {m.shop_my_subscriptions()}
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
        {m.shop_no_subscriptions()}
      </p>
    {:else}
      <div class="flex flex-col gap-1 sm:gap-2">
        {#each subscriptions as subscription (subscription.id)}
          {@const hasParts = stateValues(parts).some(
            (p) => p.shop == subscription.shop_id,
          )}
          {#snippet menu()}
            {#if subscription.status === "pending"}
              <DropdownItem onclick={() => cancelSubscription(subscription)}>
                {m.action_cancel()}
              </DropdownItem>
            {:else if subscription.status === "active"}
              <DropdownItem>
                {m.shop_register_gear({ category: getCategory()!.name })}
              </DropdownItem>

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

              <DropdownItem
                disabled={hasParts}
                title={hasParts ? m.shop_unsubscribe_disabled() : undefined}
                onclick={() =>
                  startConfirmation(subscription.id!, "unsubscribe")}
              >
                {m.shop_unsubscribe()}
              </DropdownItem>
            {:else if subscription.status === "rejected"}
              <DropdownItem
                onclick={() => startConfirmation(subscription.id!, "delete")}
              >
                {m.action_delete()}
              </DropdownItem>
            {/if}
          {/snippet}

          {#snippet body()}
            <div class="flex flex-col gap-2 min-w-48">
              <p class="text-sm text-gray-600 dark:text-gray-400">
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
          {/snippet}

          <SubscriptionCard
            {subscription}
            menu={subscription.status === "pending" ||
            subscription.status === "active" ||
            subscription.status === "rejected"
              ? menu
              : undefined}
            body={confirmingAction?.id === subscription.id ? body : undefined}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<ShopSearch {subscriptions} />
