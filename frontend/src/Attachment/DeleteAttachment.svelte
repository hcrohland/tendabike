<script lang="ts">
  import { Attachment } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { fmtDate } from "../lib/store";
  import { types } from "../lib/types";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";

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
    Do you really want to remove the {types[part.what].name}
    {part.name}
    from
    {$parts[attachment.gear].name} at {fmtDate(attachment.attached)}?
  {/snippet}
  {#snippet footer()}
    <Buttons bind:open label={"Confirm"} />
  {/snippet}
</Modal>
