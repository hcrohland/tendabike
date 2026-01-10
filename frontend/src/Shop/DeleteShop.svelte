<script lang="ts">
  import { Button } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { shops, type Shop } from "../lib/shop";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let shop = $state<Shop | undefined>(undefined);

  async function onaction() {
    if (shop) {
      await shop.delete().then(() => shops.deleteItem(shop!.id));
    }
    open = false;
  }

  export function start(g: Shop) {
    shop = g;
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    Delete Shop
  {/snippet}

  <div class="space-y-4">
    <p class="text-gray-700 dark:text-gray-300">
      Are you sure you want to delete <strong>{shop?.name}</strong>?
    </p>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      You can only delete shops that have no bikes assigned. This action cannot
      be undone.
    </p>
  </div>

  {#snippet footer()}
    <Button color="alternative" onclick={() => (open = false)}>Cancel</Button>
    <Button color="red" onclick={onaction}>Delete</Button>
  {/snippet}
</Modal>

{@render children?.()}
