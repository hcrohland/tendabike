<script lang="ts">
  import { fmtDate } from "../lib/store";
  import { Part } from "../lib/part";
  import { types } from "../lib/types";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

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
    {m.recoverpart_header({
      type: types[part.what].localizedName(),
      name: part.name,
      vendor: part.vendor,
      model: part.model,
    })}
  {/snippet}

  {m.recoverpart_binned_on({ date: fmtDate(part.disposed_at) })}

  {#snippet footer()}
    <Buttons bind:open label={m.action_recover()} />
  {/snippet}
</Modal>
