<script lang="ts">
  import { Button } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import Modal from "../Widgets/Modal.svelte";
  import type { Garage } from "../lib/garage";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let garage = $state<Garage | undefined>(undefined);

  async function onaction() {
    if (garage) {
      await garage.delete();
    }
    open = false;
  }

  export function start(g: Garage) {
    garage = g;
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    Delete Garage
  {/snippet}

  <div class="space-y-4">
    <p class="text-gray-700 dark:text-gray-300">
      Are you sure you want to delete <strong>{garage?.name}</strong>?
    </p>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      You can only delete garages that have no bikes assigned. This action
      cannot be undone.
    </p>
  </div>

  {#snippet footer()}
    <Button color="alternative" onclick={() => (open = false)}>Cancel</Button>
    <Button color="red" onclick={onaction}>Delete</Button>
  {/snippet}
</Modal>

{@render children?.()}
