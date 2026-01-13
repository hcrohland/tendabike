<script lang="ts">
  import { Attachment } from "../lib/attachment";
  import { user } from "../lib/user";
  import { types, Type } from "../lib/types";
  import { parts, Part } from "../lib/part";
  import NewForm from "../Part/PartForm.svelte";
  import Dispose from "../Widgets/Dispose.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import Modal from "../Widgets/Modal.svelte";

  let part: any = $state();
  let oldpart: Part;
  let type = $state<Type>();
  let prefix = $state("");
  let gear: number = $state(0);
  let hook: number;
  let dispose = $state(false);
  let open = $state(false);
  let mindate = $state(new Date());
  let single = $state(false);

  async function attachPart(p: Part | void) {
    if (!p) throw "Replace: update part did fail";
    await p.attach(p.purchase, !single, gear, hook);

    if (dispose) {
      await oldpart.dispose(p.purchase, !single);
    }
  }

  async function onaction() {
    await new Part(part).create().then(attachPart);
    open = false;
  }

  export const start = (attl: Attachment) => {
    oldpart = $parts[attl.part_id];
    hook = attl.hook;
    gear = attl.gear;
    mindate = attl.attached;
    type = oldpart.type();
    prefix = types[attl.hook].prefix;
    single = !type.is_hook();
    part = {
      ...new Part({
        owner: $user && $user.id,
        what: oldpart.what,
        name: oldpart.name,
        vendor: oldpart.vendor,
        model: oldpart.model,
        purchase: attl.isDetached() ? attl.detached : new Date(),
      }),
    };
    dispose = false;
    open = true;
  };
</script>

<Modal bind:open {onaction}>
  {#snippet header()}
    New {prefix}
    {type!.name} for {$parts[gear].name}
  {/snippet}
  <NewForm {type} bind:part {mindate} />
  {#if type!.is_hook()}
    <Switch bind:checked={single}>Keep all attached parts</Switch>
  {/if}
  {#if single}
    <Dispose bind:dispose>old {type!.name}</Dispose>
  {/if}

  {#snippet footer()}
    <Buttons bind:open label={"Replace"} />
  {/snippet}
</Modal>
