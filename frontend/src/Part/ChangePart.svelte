<script module lang="ts">
  import { Type, types } from "../lib/types";
  import { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    maxdate: Date | undefined;
    part: any;
    type: Type | undefined;
  }>({
    open: false,
    maxdate: undefined,
    part: undefined,
    // `types` is only populated once getTypes() has run, which is after
    // this module is imported; the value is replaced by start().
    type: types?.[0],
  });

  export const changePart = (p: Part) => {
    modal.part = { ...p };
    modal.type = p.type();
    modal.maxdate = p.firstEvent();
    modal.open = true;
  };
</script>

<script lang="ts">
  import { handleError } from "../lib/store";
  import NewForm from "./PartForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    try {
      await new Part(modal.part).update();
    } catch (e: any) {
      handleError(e);
    }

    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.changepart_header({ type: modal.type?.localizedName() ?? "" })}
  {/snippet}
  <NewForm type={modal.type} bind:part={modal.part} maxdate={modal.maxdate} />

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_change()} />
  {/snippet}
</Modal>
