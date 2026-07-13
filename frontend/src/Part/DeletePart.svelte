<script lang="ts">
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  let part = $state(new Part({}));

  let open = $state(false);

  async function onaction() {
    await part.delete();
    open = false;
  }

  export const start = (p: Part) => {
    part = p;
    open = true;
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    {m.deletepart_header({ name: part.name })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_delete()} />
  {/snippet}
</Modal>
