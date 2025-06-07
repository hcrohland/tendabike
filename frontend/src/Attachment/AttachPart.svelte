<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    ModalFooter,
  } from "@sveltestrap/sveltestrap";
  import { types } from "../lib/types";
  import AttachForm from "./AttachForm.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";

  let part: Part | undefined;
  let isOpen = false;
  let disabled = true;
  let time: Date;
  let gear: number;
  let hook: number;
  const toggle = () => {
    part = undefined;
    isOpen = false;
  };

  async function action() {
    disabled = true;

    await part!.attach(time, true, gear!, hook);
    isOpen = false;
  }

  export const attachPart = (p: Part) => {
    part = p;
    isOpen = true;
  };
</script>

{#if part}
  <Modal {isOpen} {toggle} backdrop={false}>
    <ModalHeader {toggle}>
      Attach {types[part.what].name}
      {part.name}
      {part.vendor}
      {part.model}
    </ModalHeader>
    <form on:submit|preventDefault={action}>
      <ModalBody>
        <AttachForm bind:time bind:gear bind:hook bind:disabled {part} />
      </ModalBody>
      <ModalFooter>
        <Buttons {toggle} {disabled} label={"Attach"} />
      </ModalFooter>
    </form>
  </Modal>
{/if}
