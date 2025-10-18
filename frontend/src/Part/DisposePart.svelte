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

  let open = false;
  let last: Attachment | undefined;
  let part: Part;
  let name: String;
  let detach: boolean;
  let dispose: boolean;
  let mindate: Date;
  let date: Date;
  let all: boolean;

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
    name = part.type().name;
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

  $: label = detach ? "Detach " : "Dispose ";
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    {label}
    {name}
    {part.name}
  {/snippet}
  <div>
    <ButtonGroup>
      <InputAddon>At</InputAddon>
      <DateTime bind:date {mindate} />
    </ButtonGroup>
  </div>
  <Switch bind:checked={all}>{label} all attached parts</Switch>
  {#if detach}
    <Dispose bind:dispose>{name} when detached</Dispose>
  {/if}

  {#snippet footer()}
    <Buttons bind:open {label} />
  {/snippet}
</Modal>
