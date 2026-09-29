<script module lang="ts">
  import { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    part: Part | undefined;
    time: Date;
    gear: number | undefined;
    hook: number | undefined;
  }>({
    open: false,
    part: undefined,
    time: new Date(),
    gear: undefined,
    hook: undefined,
  });

  export const attachPart = (p: Part) => {
    modal.part = p;
    modal.time = new Date();
    modal.gear = undefined;
    modal.hook = undefined;
    modal.open = true;
  };
</script>

<script lang="ts">
  import { types } from "../lib/types";
  import AttachForm from "./AttachForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await modal.part!.attach(modal.time!, true, modal.gear!, modal.hook!);
    modal.open = false;
  }
</script>

{#if modal.part}
  <Modal bind:open={modal.open} {onaction}>
    {#snippet header()}
      {m.attachpart_header({
        type: types[modal.part!.what].localizedName(),
        name: modal.part!.name,
        vendor: modal.part!.vendor,
        model: modal.part!.model,
      })}
    {/snippet}
    <AttachForm
      bind:time={modal.time}
      bind:gear={modal.gear}
      bind:hook={modal.hook}
      part={modal.part}
    />

    {#snippet footer()}
      <Buttons bind:open={modal.open} label={m.action_attach()} />
    {/snippet}
  </Modal>
{/if}
