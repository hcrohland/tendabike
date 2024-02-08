<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "@sveltestrap/sveltestrap";
  import ModalFooter from "./ModalFooter.svelte";
  import { types } from "../lib/store";
  import { AttEvent } from "../lib/types";
  import AttachForm from "./AttachForm.svelte";
  import { Part } from "../Part/part";

  let attach: AttEvent;
  let part: Part | undefined;
  let isOpen = false;
  let disabled = true;
  const toggle = () => {
    part = undefined;
    isOpen = false;
  };

  async function action() {
    disabled = true;

    await attach.attach();
    isOpen = false;
  }

  export const attachPart = (p: Part) => {
    part = p;
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  {#if part}
    <ModalHeader {toggle}>
      Attach {types[part.what].name}
      {part.name}
      {part.vendor}
      {part.model}
    </ModalHeader>
    <ModalBody>
      <AttachForm bind:attach bind:disabled {part} />
    </ModalBody>
  {:else}
    Error: part is not defined
  {/if}
  <ModalFooter {toggle} {action} {disabled} button={"Attach"} />
</Modal>
