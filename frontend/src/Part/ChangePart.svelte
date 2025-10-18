<script lang="ts">
  import { handleError } from "../lib/store";
  import { Type, types } from "../lib/types";
  import { attachments } from "../lib/attachment";
  import NewForm from "./PartForm.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import { activities } from "../lib/activity";
  import Modal from "../Widgets/Modal.svelte";

  let open = $state(false);

  let maxdate: Date | undefined = $state();
  let part: any = $state();
  let type: Type = $state(types[0]);

  async function onaction() {
    try {
      await new Part(part).update();
    } catch (e: any) {
      handleError(e);
    }

    open = false;
  }

  export const start = (p: Part) => {
    part = { ...p };
    type = p.type();
    maxdate = p.firstEvent($activities, $attachments);
    open = true;
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    Change {type.name} details
  {/snippet}
  <NewForm {type} bind:part {maxdate} />

  {#snippet footer()}
    <Buttons bind:open label="Change" />
  {/snippet}
</Modal>
