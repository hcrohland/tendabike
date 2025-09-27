<script lang="ts">
  import { Button, Modal } from "flowbite-svelte";
  import NewForm from "./PartForm.svelte";
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import { user } from "../lib/store";

  interface Props {
    type: Type;
  }

  let { type }: Props = $props();
  let part = $state(new Part({}));
  let open = $state(false);

  async function savePart() {
    let newpart = new Part({
      owner: $user && $user.id,
      what: type.id,
      purchase: part.purchase,
      last_used: part.purchase,
      name: part.name,
      vendor: part.vendor,
      model: part.model,
    });
    await newpart.create();
  }
</script>

<Button
  size="xs"
  color="alternative"
  class="p-1 cursor-pointer"
  onclick={() => (open = true)}>New</Button
>
<Modal form bind:open onaction={savePart} onclose={() => (part = new Part({}))}>
  {#snippet header()}
    New {type.name}
  {/snippet}
  <NewForm {type} bind:part />
  {#snippet footer()}
    <Button type="submit" value="create">Create</Button>
  {/snippet}
</Modal>
