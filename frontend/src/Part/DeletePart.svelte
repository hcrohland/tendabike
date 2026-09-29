<script module lang="ts">
  import { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    part: Part;
  }>({
    open: false,
    part: new Part({}),
  });

  export const deletePart = (p: Part) => {
    modal.part = p;
    modal.open = true;
  };
</script>

<script lang="ts">
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.part.delete();
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.deletepart_header({ name: modal.part.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_delete()} />
  {/snippet}
</Modal>
