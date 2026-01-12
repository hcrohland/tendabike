<script lang="ts">
  import { Checkbox, Input, Label, Textarea } from "flowbite-svelte";
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
  let auto_approve = $state(false);
  let title = $state("Create Shop");

  async function onaction() {
    shop.name = name;
    shop.description = description || undefined;
    shop.auto_approve = auto_approve;

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
      auto_approve = g.auto_approve;
      title = "Edit Shop";
    } else {
      shop = new Shop({});
      name = "";
      description = "";
      auto_approve = false;
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
    <div>
      <Label for="auto_approve" class="mb-2"
        >If registration requests are automatically approved</Label
      >
      <Checkbox id="auto_approve" bind:checked={auto_approve} />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label="Save" />
  {/snippet}
</Modal>

{@render children?.()}
