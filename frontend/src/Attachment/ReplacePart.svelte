<script lang="ts">
  import { Modal, ModalHeader, ModalBody, ModalFooter } from "flowbite-svelte";
  import { Attachment } from "../lib/attachment";
  import { user } from "../lib/store";
  import { types, Type } from "../lib/types";
  import { parts, Part } from "../lib/part";
  import NewForm from "../Part/PartForm.svelte";
  import Dispose from "../Widgets/Dispose.svelte";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";

  let part: Part, oldpart: Part, newpart: Part;
  let type: Type;
  let prefix: string;
  let gear: number;
  let hook: number;
  let disabled = true;
  let dispose = false;
  let isOpen = false;
  let mindate: Date;
  let single = false;

  const toggle = () => (isOpen = false);

  async function attachPart(part: Part | void) {
    if (!part) throw "Replace: update part did fail";
    await part.attach(part.purchase, !single, gear, hook);

    if (dispose) {
      await oldpart.dispose(part.purchase, !single);
    }
  }

  async function action() {
    disabled = true;
    await newpart.create().then(attachPart);
    isOpen = false;
    isOpen = false;
  }

  export const replacePart = (attl: Attachment) => {
    oldpart = $parts[attl.part_id];
    hook = attl.hook;
    gear = attl.gear;
    mindate = attl.attached;
    type = oldpart.type();
    prefix = types[attl.hook].prefix;
    single = !type.is_hook();
    part = new Part({
      owner: $user && $user.id,
      what: oldpart.what,
      name: oldpart.name,
      vendor: oldpart.vendor,
      model: oldpart.model,
      purchase: attl.isDetached() ? attl.detached : new Date(),
    });
    dispose = false;
    isOpen = true;
  };

  const setPart = (e: CustomEvent<Part>) => {
    newpart = new Part(e.detail);
    disabled = false;
  };
</script>

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>
    New {prefix}
    {type.name} for {$parts[gear].name}
  </ModalHeader>
  <form on:submit|preventDefault={action}>
    <ModalBody>
      <NewForm {type} {part} {mindate} on:change={setPart} />
      {#if type.is_hook()}
        <Switch bind:checked={single}>Keep all attached parts</Switch>
      {/if}
      {#if single}
        <Dispose bind:dispose>old {type.name}</Dispose>
      {/if}
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Replace"} />
    </ModalFooter>
  </form>
</Modal>
