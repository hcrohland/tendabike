<script lang="ts">
  import { Attachment } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  let attachment = $state(new Attachment({}));
  let part = $state(new Part({}));
  let open = $state(false);

  async function onaction() {
    await part.detach(attachment.attached, true);
    open = false;
  }

  export const start = (a: Attachment) => {
    attachment = a;
    part = $parts[a.part_id];
    open = true;
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    {m.deleteattachment_header({
      type: types[part.what].localizedName(),
      name: part.name,
      gear: $parts[attachment.gear].name,
      date: fmtDate(attachment.attached),
    })}
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={m.action_confirm()} />
  {/snippet}
</Modal>
