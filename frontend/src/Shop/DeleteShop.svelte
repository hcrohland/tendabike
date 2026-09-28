<script module lang="ts">
  import type { Shop } from "../lib/shop";

  const modal = $state<{
    open: boolean;
    shop: Shop | undefined;
  }>({
    open: false,
    shop: undefined,
  });

  export function start(g: Shop) {
    modal.shop = g;
    modal.open = true;
  }
</script>

<script lang="ts">
  import { Button } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import Modal from "../Widgets/Modal.svelte";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  async function onaction() {
    if (modal.shop) {
      await modal.shop.delete();
    }
    modal.open = false;
  }
</script>

<Modal size="sm" bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.shop_delete()}
  {/snippet}

  <div class="space-y-4">
    <p class="text-gray-700 dark:text-gray-300">
      {m.shop_delete_confirm({ name: modal.shop?.name ?? "" })}
    </p>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {m.shop_delete_hint()}
    </p>
  </div>

  {#snippet footer()}
    <Button color="alternative" onclick={() => (modal.open = false)}>
      {m.action_cancel()}
    </Button>
    <Button color="red" onclick={onaction}>
      {m.action_delete()}
    </Button>
  {/snippet}
</Modal>

{@render children?.()}
