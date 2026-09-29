<script module lang="ts">
  import { Attachment } from "../lib/attachment";
  import { Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    last: Attachment | undefined;
    part: Part;
    typeName: string;
    detach: boolean;
    dispose: boolean;
    mindate: Date;
    date: Date;
    all: boolean;
    hook: boolean;
  }>({
    open: false,
    last: undefined,
    part: new Part({}),
    typeName: "",
    detach: false,
    dispose: false,
    mindate: new Date(),
    date: new Date(),
    all: false,
    hook: false,
  });

  export const disposePart = (p: Part, last_attachment?: Attachment) => {
    modal.part = p;
    let type = modal.part.type();
    modal.typeName = type.localizedName();
    modal.hook = type.is_hook();
    modal.last = last_attachment;

    if (last_attachment) {
      if (last_attachment.isDetached()) {
        modal.detach = false;
        modal.dispose = true;
        modal.mindate = last_attachment.detached;
      } else {
        modal.detach = true;
        modal.dispose = false;
        modal.mindate = last_attachment.attached;
      }
    } else {
      modal.mindate = modal.part.purchase;
      modal.detach = false;
      modal.dispose = true;
    }
    modal.all = true;
    modal.date = new Date();
    modal.open = true;
  };
</script>

<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import { handleError } from "../lib/store";
  import Dispose from "../Widgets/Dispose.svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  let action = $derived(modal.detach ? m.action_detach() : m.action_dispose());

  async function onaction() {
    try {
      if (modal.detach) {
        await modal.part.detach(modal.date, modal.all);
      }
      if (modal.dispose) {
        await modal.part.dispose(modal.date, modal.all);
      }
    } catch (e: any) {
      handleError(e);
    }
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.dispose_question({ name: modal.typeName + " " + modal.part.name })}
  {/snippet}
  <div>
    <ButtonGroup>
      <InputAddon>{m.attachform_at()}</InputAddon>
      <DateTime bind:date={modal.date} mindate={modal.mindate} />
    </ButtonGroup>
  </div>
  {#if modal.hook}
    <Switch bind:checked={modal.all}>
      {m.disposepart_all({ action })}
    </Switch>
  {/if}
  {#if modal.detach}
    <Dispose
      bind:dispose={modal.dispose}
      name={m.disposepart_when_detached({ type: modal.typeName })}
    />
  {/if}

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={action} />
  {/snippet}
</Modal>
