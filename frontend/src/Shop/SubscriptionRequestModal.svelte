<script module lang="ts">
  import type { Shop } from "../lib/shop";

  const modal = $state<{
    open: boolean;
    shop: Shop | undefined;
    message: string;
  }>({
    open: false,
    shop: undefined,
    message: "",
  });

  export function start(g: Shop) {
    modal.shop = g;
    modal.message = "";
    modal.open = true;
  }
</script>

<script lang="ts">
  import { Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { handleError } from "../lib/store";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  async function onaction() {
    if (!modal.shop?.id) return;

    try {
      await modal.shop.requestSubscription(modal.message || undefined);

      // Notify other components that subscriptions have been updated
      window.dispatchEvent(new CustomEvent("subscription-updated"));

      modal.open = false;
      modal.message = "";
    } catch (error) {
      handleError(error as Error);
    }
  }
</script>

<Modal size="sm" bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.shop_request_subscription()}
  {/snippet}

  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {m.shop_request_subscription_description({
        name: modal.shop?.name ?? "",
      })}
    </p>

    <div>
      <Label for="message" class="mb-2">
        {m.shop_request_message()}
      </Label>

      <Textarea
        id="message"
        bind:value={modal.message}
        placeholder={m.shop_request_message_placeholder()}
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.shop_send_request()} />
  {/snippet}
</Modal>

{@render children?.()}
