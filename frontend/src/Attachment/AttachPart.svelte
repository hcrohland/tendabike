<script lang="ts">
  import { types } from "../lib/types";
  import AttachForm from "./AttachForm.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  let part: Part | undefined = $state();
  let open = $state(false);
  let time = $state<Date>();
  let gear = $state<number>();
  let hook = $state<number>();

  async function onaction() {
    await part!.attach(time!, true, gear!, hook!);
    open = false;
  }

  export const start = (p: Part) => {
    part = p;
    time = undefined;
    gear = undefined;
    hook = undefined;
    open = true;
  };
</script>

{#if part}
  <Modal bind:open {onaction}>
    {#snippet header()}
      {m.attachpart_header({
        type: types[part!.what].localizedName(),
        name: part!.name,
        vendor: part!.vendor,
        model: part!.model,
      })}
    {/snippet}
    <AttachForm bind:time bind:gear bind:hook {part} />

    {#snippet footer()}
      <Buttons bind:open label={m.action_attach()} />
    {/snippet}
  </Modal>
{/if}
