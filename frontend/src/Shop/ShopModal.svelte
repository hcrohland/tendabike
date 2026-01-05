<script lang="ts">
  import { Input, Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { Shop } from "../lib/shop";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let shop = $state(new Shop({}));
  let name = $state("");
  let description = $state("");
  let title = $state("Create Shop");

  async function onaction() {
    shop.name = name;
    shop.description = description || undefined;

    if (shop.id) {
      await shop.update();
    } else {
      await shop.create();
    }

    open = false;
  }

  export function start(g?: Shop) {
    if (g) {
      shop = g;
      name = g.name;
      description = g.description || "";
      title = "Edit Shop";
    } else {
      shop = new Shop({});
      name = "";
      description = "";
      title = "Create Shop";
    }
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    {title}
  {/snippet}

  <div class="space-y-4">
    <div>
      <Label for="name" class="mb-2">Shop Name</Label>
      <Input
        id="name"
        type="text"
        bind:value={name}
        placeholder="My Bike Shop"
        autofocus
        required
      />
    </div>

    <div>
      <Label for="description" class="mb-2">Description (optional)</Label>
      <Textarea
        id="description"
        bind:value={description}
        placeholder="Describe your shop..."
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label="Save" />
  {/snippet}
</Modal>

{@render children?.()}
