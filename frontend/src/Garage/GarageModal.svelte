<script lang="ts">
  import { Input, Label, Textarea } from "flowbite-svelte";
  import type { Snippet } from "svelte";
  import Modal from "../Widgets/Modal.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { Garage } from "../lib/garage";

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let open = $state(false);
  let garage = $state(new Garage({}));
  let name = $state("");
  let description = $state("");
  let title = $state("Create Garage");

  async function onaction() {
    garage.name = name;
    garage.description = description || undefined;

    if (garage.id) {
      await garage.update();
    } else {
      await garage.create();
    }

    open = false;
  }

  export function start(g?: Garage) {
    if (g) {
      garage = g;
      name = g.name;
      description = g.description || "";
      title = "Edit Garage";
    } else {
      garage = new Garage({});
      name = "";
      description = "";
      title = "Create Garage";
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
      <Label for="name" class="mb-2">Garage Name</Label>
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
        placeholder="Describe your garage..."
        rows={3}
      />
    </div>
  </div>

  {#snippet footer()}
    <Buttons bind:open label="Save" />
  {/snippet}
</Modal>

{@render children?.()}
