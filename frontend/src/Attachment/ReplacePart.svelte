<script module lang="ts">
  import { Attachment } from "../lib/attachment";
  import { getUser } from "../lib/user";
  import { types, Type } from "../lib/types";
  import { parts, Part } from "../lib/part";

  const modal = $state<{
    open: boolean;
    part: any;
    oldpart: Part;
    type: Type | undefined;
    prefix: string;
    gear: number;
    hook: number;
    dispose: boolean;
    mindate: Date;
    single: boolean;
  }>({
    open: false,
    part: undefined,
    oldpart: new Part({}),
    type: undefined,
    prefix: "",
    gear: 0,
    hook: 0,
    dispose: false,
    mindate: new Date(),
    single: false,
  });

  export const replacePart = (attl: Attachment) => {
    modal.oldpart = parts[attl.part_id];
    modal.hook = attl.hook;
    modal.gear = attl.gear;
    modal.mindate = attl.attached;
    modal.type = modal.oldpart.type();
    modal.prefix = types[attl.hook].prefix;
    modal.single = !modal.type.is_hook();
    modal.part = {
      ...new Part({
        owner: getUser()?.id,
        what: modal.oldpart.what,
        name: modal.oldpart.name,
        vendor: modal.oldpart.vendor,
        model: modal.oldpart.model,
        purchase: attl.isDetached() ? attl.detached : new Date(),
      }),
    };
    modal.dispose = false;
    modal.open = true;
  };
</script>

<script lang="ts">
  import NewForm from "../Part/PartForm.svelte";
  import Dispose from "../Widgets/Dispose.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";
  import { m } from "../../paraglide/messages";

  async function attachPart(p: Part | void) {
    if (!p) throw "Replace: update part did fail";
    await p.attach(p.purchase, !modal.single, modal.gear, modal.hook);

    if (modal.dispose) {
      await modal.oldpart.dispose(p.purchase, !modal.single);
    }
  }

  async function onaction() {
    await new Part(modal.part).create().then(attachPart);
    modal.open = false;
  }
</script>

<Modal bind:open={modal.open} {onaction}>
  {#snippet header()}
    {m.replacepart_header({
      type: modal.type!.labelWithPosition(modal.prefix),
      gear: parts[modal.gear].name,
    })}
  {/snippet}
  <NewForm type={modal.type} bind:part={modal.part} mindate={modal.mindate} />
  {#if modal.type!.is_hook()}
    <Switch bind:checked={modal.single}>{m.installpart_keep_attached()}</Switch>
  {/if}
  {#if modal.single}
    <Dispose
      bind:dispose={modal.dispose}
      name={modal.type!.localizedOldAccusative()}
    />
  {/if}

  {#snippet footer()}
    <Buttons bind:open={modal.open} label={m.action_replace()} />
  {/snippet}
</Modal>
