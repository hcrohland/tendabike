<script module lang="ts">
  import { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    part: Part;
  }>({
    open: false,
    part: new Part({}),
  });

  export const recoverPart = (p: Part) => {
    modal.part = p;
    modal.open = true;
  };
</script>

<script lang="ts">
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.part.recover(true);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.recoverpart_header({
      type: types[modal.part.what].localizedName(),
      name: modal.part.name,
      vendor: modal.part.vendor,
      model: modal.part.model,
    })}
  {/snippet}

  {m.recoverpart_binned_on({ date: fmtDate(modal.part.disposed_at) })}

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_recover()} />
  {/snippet}
</Modal>
