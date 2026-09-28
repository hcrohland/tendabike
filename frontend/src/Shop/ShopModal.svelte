<script module lang="ts">
  import { Shop } from "../lib/shop";

  const modal = $state<{
    open: boolean;
    shop: Shop;
    name: string;
    description: string;
    auto_approve: boolean;
    editing: boolean;
  }>({
    open: false,
    shop: new Shop({}),
    name: "",
    description: "",
    auto_approve: false,
    editing: false,
  });

  export function start(g?: Shop) {
    if (g) {
      modal.shop = g;
      modal.name = g.name;
      modal.description = g.description || "";
      modal.auto_approve = g.auto_approve;
      modal.editing = true;
    } else {
      modal.shop = new Shop({});
      modal.name = "";
      modal.description = "";
      modal.auto_approve = false;
      modal.editing = false;
    }
    modal.open = true;
  }
</script>

<script lang="ts">
  import { Checkbox, Input, Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  async function onaction() {
    modal.shop.name = modal.name;
    modal.shop.description = modal.description || undefined;
    modal.shop.auto_approve = modal.auto_approve;

    if (modal.shop.id) {
      await modal.shop.update();
    } else {
      await modal.shop.create();
    }

    modal.open = false;
  }
</script>

<Modal size="sm" bind:open={modal.open} {onaction}>
  {#snippet header()}
    {modal.editing ? m.shop_edit() : m.shop_create()}
  {/snippet}

  <div class="space-y-4">
    <div>
      <Label for="name" class="mb-2">{m.shop_name()}</Label>
      <Input
        id="name"
        type="text"
        bind:value={modal.name}
        placeholder={m.shop_name_placeholder()}
        autofocus
        required
      />
    </div>

    <div>
      <Label for="description" class="mb-2">
        {m.shop_description()}
      </Label>
      <Textarea
        id="description"
        bind:value={modal.description}
        placeholder={m.shop_description_placeholder()}
        rows={3}
      />
    </div>

    <div>
      <Label for="auto_approve" class="mb-2">
        {m.shop_autoapprove()}
      </Label>
      <Checkbox id="auto_approve" bind:checked={modal.auto_approve} />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.gearcard_save()} />
  {/snippet}
</Modal>

{@render children?.()}
