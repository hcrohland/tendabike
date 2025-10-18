<script lang="ts">
  import { fmtDate } from "../lib/store";
  import { Part } from "../lib/part";
  import { types } from "../lib/types";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

  let part = $state(new Part({}));
  let open = $state(false);

  async function onaction() {
    await part.recover(true);
    open = false;
  }

  export const start = (p: Part) => {
    part = p;
    open = true;
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    Do you really have {types[part.what].name}
    {part.name}
    {part.vendor}
    {part.model} back?
  {/snippet}

  You binned it on {fmtDate(part.disposed_at)}

  {#snippet footer()}
    <Buttons bind:open label={"Recover"} />
  {/snippet}
</Modal>
