<script lang="ts">
  import { Button } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import Modal from "../Widgets/Modal.svelte";
  import { type Shop } from "../lib/shop";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let shop = $state<Shop | undefined>(undefined);

  async function onaction() {
    if (shop) {
      await shop.delete();
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
    {m.shop_delete()}
  {/snippet}

  <div class="space-y-4">
    <p class="text-gray-700 dark:text-gray-300">
      {m.shop_delete_confirm({ name: shop?.name ?? "" })}
    </p>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {m.shop_delete_hint()}
    </p>
  </div>

  {#snippet footer()}
    <Button color="alternative" onclick={() => (open = false)}>
      {m.action_cancel()}
    </Button>
    <Button color="red" onclick={onaction}>
      {m.action_delete()}
    </Button>
  {/snippet}
</Modal>

{@render children?.()}
