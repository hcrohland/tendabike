<script module lang="ts">
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import { getUser } from "../lib/user";

  const modal = $state<{
    open: boolean;
    type: Type | undefined;
    part: any;
  }>({
    open: false,
    type: undefined,
    part: undefined,
  });

  export function start(t: Type) {
    modal.type = t;
    modal.part = { ...new Part({ owner: getUser()?.id, what: t.id }) };
    modal.open = true;
  }
</script>

<script lang="ts">
  import Modal from "../Widgets/Modal.svelte";
  import NewForm from "./PartForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import { m } from "../../paraglide/messages";

  async function onaction() {
    await new Part(modal.part).create();
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.newpart_header({ type: modal.type!.localizedName() })}
  {/snippet}
  <NewForm type={modal.type} bind:part={modal.part} />
  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_create()} />
  {/snippet}
</Modal>
