<script lang="ts">
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

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
    Do you really want to delete Part <br />
    "{part.name}"?
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label="Delete" />
  {/snippet}
</Modal>
