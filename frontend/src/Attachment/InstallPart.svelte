<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    ModalFooter,
    InputGroup,
    InputGroupText,
  } from "@sveltestrap/sveltestrap";
  import { Type } from "../lib/types";
  import { user } from "../lib/store";
  import NewForm from "../Part/PartForm.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import { filterValues } from "../lib/mapable";
  import { Part } from "../lib/part";
  import { AttEvent, attachments } from "../lib/attachment";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";

  let part: Part, newpart: Part;
  let gear: Part;
  let type: Type | undefined;
  let hook: number;
  let disabled = true;
  let isOpen = false;
  let single = true;

  const toggle = () => (isOpen = false);
  export const installPart = (g: Part) => {
    gear = g;
    part = new Part({
      owner: $user && $user.id,
    });
    disabled = true;
    type = undefined;
    isOpen = true;
  };

  async function attachPart(part: Part | void) {
    if (!part) return;
    await new AttEvent(part.id, part.purchase, gear.id, hook, !single).attach();
  }

  async function action() {
    disabled = true;
    await newpart.create().then(attachPart);
    isOpen = false;
  }

  function guessDate(g: Part, t: Type, hook: number) {
    if (!t) return new Date();
    let last = filterValues(
      $attachments,
      (a) => a.gear == g.id && a.what == t.id && a.hook == hook,
    );
    if (last.length) {
      // It is a replacement
      return new Date();
    } else {
      // It is the first part of that type
      return g.purchase;
    }
  }

  part = new Part({
    owner: $user && $user.id,
    purchase: new Date(),
    last_used: new Date(),
  });

  const setType = (e: CustomEvent<{ type: Type; hook: number }>) => {
    type = e.detail.type;
    hook = e.detail.hook;
    part.what = type.id;
    part.purchase = guessDate(gear, type, hook);
  };
  const setPart = (e: CustomEvent<Part>) => {
    newpart = new Part(e.detail);
    disabled = false;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <form on:submit|preventDefault={action}>
    <ModalHeader {toggle}>
      <InputGroup class="col-md-12">
        <InputGroupText>New</InputGroupText>
        <TypeForm on:change={setType} />
        <InputGroupText>of {gear.name}</InputGroupText>
      </InputGroup>
    </ModalHeader>
    <ModalBody>
      <NewForm {type} {part} mindate={gear.purchase} on:change={setPart} />
      {#if type?.is_hook()}
        <Switch bind:checked={single}>Keep all attached parts</Switch>
      {/if}
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Install"} />
    </ModalFooter>
  </form>
</Modal>
