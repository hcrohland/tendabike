<script lang="ts">
  import { Modal, ModalBody, ModalHeader, ModalFooter } from "flowbite-svelte";
  import { handleError } from "../lib/store";
  import { Type } from "../lib/types";
  import { attachments } from "../lib/attachment";
  import NewForm from "./PartForm.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import { activities } from "../lib/activity";

  let isOpen = false;
  let disabled = true;

  let start: Date | undefined;
  let part: Part;
  let newpart: Part;
  let type: Type;

  async function savePart() {
    try {
      disabled = true;
      await newpart.update();
    } catch (e: any) {
      handleError(e);
    }

    isOpen = false;
  }

  export const changePart = (p: Part) => {
    part = p;
    type = part.type();
    start = part.firstEvent($activities, $attachments);
    disabled = true;
    isOpen = true;
  };

  const toggle = () => (isOpen = false);
  const setPart = (e: CustomEvent<Part>) => {
    newpart = new Part(e.detail);
    disabled = false;
  };
</script>

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>Change {type.name} details</ModalHeader>
  <form on:submit|preventDefault={savePart}>
    <ModalBody>
      <NewForm {type} {part} on:change={setPart} maxdate={start} />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Change"} />
    </ModalFooter>
  </form>
</Modal>
