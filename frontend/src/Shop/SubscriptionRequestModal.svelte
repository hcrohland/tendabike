<script lang="ts">
  import { Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
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
    Request Shop Subscription
  {/snippet}

  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">
      Request to subscribe to <strong>{shop?.name}</strong>. Once approved, you
      can register any of your bikes to this shop.
    </p>

    <div>
      <Label for="message" class="mb-2">Message (optional)</Label>
      <Textarea
        id="message"
        bind:value={message}
        placeholder="Add a message to the shop owner..."
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label="Send Request" />
  {/snippet}
</Modal>

{@render children?.()}
