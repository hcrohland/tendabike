<script lang="ts">
  import { ButtonGroup, InputAddon } from "flowbite-svelte";
  import { handleError } from "../lib/store";
  import { Attachment } from "../lib/attachment";
  import Dispose from "../Widgets/Dispose.svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  let open = false;
  let last: Attachment | undefined;
  let part: Part;
  let typeName: string;
  let detach: boolean;
  let dispose: boolean;
  let mindate: Date;
  let date: Date;
  let all: boolean;
  let hook: boolean;

  async function onaction() {
    try {
      if (detach) {
        await part.detach(date, all);
      }
      if (dispose) {
        await part.dispose(date, all);
      }
    } catch (e: any) {
      handleError(e);
    }
    open = false;
  }

  export const start = (p: Part, last_attachment?: Attachment) => {
    part = p;
    let type = part.type();
    typeName = type.localizedName();
    hook = type.is_hook();
    last = last_attachment;

    if (last) {
      if (last.isDetached()) {
        detach = false;
        dispose = true;
        mindate = last.detached;
      } else {
        detach = true;
        dispose = false;
        mindate = last.attached;
      }
    } else {
      mindate = part.purchase;
      detach = false;
      dispose = true;
    }
    all = true;
    date = new Date();
    open = true;
  };

  $: action = detach ? m.action_detach() : m.action_dispose();
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    {m.dispose_question({ name: typeName + " " + part.name })}
  {/snippet}
  <div>
    <ButtonGroup>
      <InputAddon>{m.attachform_at()}</InputAddon>
      <DateTime bind:date {mindate} />
    </ButtonGroup>
  </div>
  {#if hook}
    <Switch bind:checked={all}>
      {m.disposepart_all({ action })}
    </Switch>
  {/if}
  {#if detach}
    <Dispose
      bind:dispose
      name={m.disposepart_when_detached({ type: typeName })}
    />
  {/if}

  {#snippet footer()}
    <Buttons bind:open label={action} />
  {/snippet}
</Modal>
