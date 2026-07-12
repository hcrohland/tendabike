<script lang="ts">
  import { Checkbox, Input, Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import * as m from "../../paraglide/messages";
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
  let editing = $state(false);

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
      editing = true;
    } else {
      shop = new Shop({});
      name = "";
      description = "";
      auto_approve = false;
      editing = false;
    }
    open = true;
  }
</script>

<Modal size="sm" bind:open {onaction}>
  {#snippet header()}
    {editing ? m.shop_edit() : m.shop_create()}
  {/snippet}

  <div class="space-y-4">
    <div>
      <Label for="name" class="mb-2">{m.shop_name()}</Label>
      <Input
        id="name"
        type="text"
        bind:value={name}
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
        bind:value={description}
        placeholder={m.shop_description_placeholder()}
        rows={3}
      />
    </div>

    <div>
      <Label for="auto_approve" class="mb-2">
        {m.shop_autoapprove()}
      </Label>
      <Checkbox id="auto_approve" bind:checked={auto_approve} />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label={m.gearcard_save()} />
  {/snippet}
</Modal>

{@render children?.()}
