<script lang="ts">
  import { Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import type { Garage } from "../lib/garage";
  import { handleError } from "../lib/store";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let garage = $state<Garage | undefined>(undefined);
  let message = $state("");

  async function onaction() {
    if (!garage?.id) return;

    try {
      await garage.requestSubscription(message || undefined);

      // Notify other components that subscriptions have been updated
      window.dispatchEvent(new CustomEvent("subscription-updated"));

      open = false;
      message = "";
    } catch (error) {
      handleError(error as Error);
    }
  }

  export function start(g: Garage) {
    garage = g;
    message = "";
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    Request Garage Subscription
  {/snippet}

  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">
      Request to subscribe to <strong>{garage?.name}</strong>. Once approved,
      you can register any of your bikes to this garage.
    </p>

    <div>
      <Label for="message" class="mb-2">Message (optional)</Label>
      <Textarea
        id="message"
        bind:value={message}
        placeholder="Add a message to the garage owner..."
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label="Send Request" />
  {/snippet}
</Modal>

{@render children?.()}
