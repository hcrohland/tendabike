<script lang="ts">
  import { Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { type Shop } from "../lib/shop";
  import { handleError } from "../lib/store";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let shop = $state<Shop | undefined>(undefined);
  let message = $state("");

  async function onaction() {
    if (!shop?.id) return;

    try {
      await shop.requestSubscription(message || undefined);

      // Notify other components that subscriptions have been updated
      window.dispatchEvent(new CustomEvent("subscription-updated"));

      open = false;
      message = "";
    } catch (error) {
      handleError(error as Error);
    }
  }

  export function start(g: Shop) {
    shop = g;
    message = "";
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    {m.shop_request_subscription()}
  {/snippet}

  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {m.shop_request_subscription_description({ name: shop?.name ?? "" })}
    </p>

    <div>
      <Label for="message" class="mb-2">
        {m.shop_request_message()}
      </Label>

      <Textarea
        id="message"
        bind:value={message}
        placeholder={m.shop_request_message_placeholder()}
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label={m.shop_send_request()} />
  {/snippet}
</Modal>

{@render children?.()}
