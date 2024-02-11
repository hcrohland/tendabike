<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    Form,
  } from "@sveltestrap/sveltestrap";
  import { Type } from "../lib/types";
  import { user } from "../lib/store";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import NewForm from "../Part/PartForm.svelte";
  import TypeForm from "../Widgets/TypeForm.svelte";
  import { filterValues } from "../lib/mapable";
  import { Part } from "../Part/part";
  import { AttEvent, attachments } from "./attachment";

  let part: Part, newpart: Part;
  let gear: Part;
  let type: Type | undefined;
  let hook: number;
  let disabled = true;
  let isOpen = false;
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
    await new AttEvent(part.id, part.purchase, gear.id, hook).attach();
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
  <ModalHeader {toggle}>
    <TypeForm {gear} on:change={setType} />
  </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} mindate={gear.purchase} on:change={setPart} />
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={"Install"} />
</Modal>
