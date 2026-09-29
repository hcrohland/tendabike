<script module lang="ts">
  import { Attachment } from "../lib/attachment";
  import { Part, parts } from "../lib/part";

  const modal = $state<{
    open: boolean;
    attachment: Attachment;
    part: Part;
  }>({
    open: false,
    attachment: new Attachment({}),
    part: new Part({}),
  });

  export const deleteAttachment = (a: Attachment) => {
    modal.attachment = a;
    modal.part = parts[a.part_id];
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
    await modal.part.detach(modal.attachment.attached, true);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.deleteattachment_header({
      type: types[modal.part.what].localizedName(),
      name: modal.part.name,
      gear: parts[modal.attachment.gear].name,
      date: fmtDate(modal.attachment.attached),
    })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_confirm()} />
  {/snippet}
</Modal>
