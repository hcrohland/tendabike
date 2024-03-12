<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
  } from "@sveltestrap/sveltestrap";
  import NewForm from "./PartForm.svelte";
  import { user } from "../lib/store";
  import { Type } from "../lib/types";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";

  let part: Part, newpart: Part;
  let type: Type;
  let isOpen = false;
  let disabled = true;

  async function savePart() {
    disabled = true;
    await newpart.create();
    isOpen = false;
  }

  export const newPart = (t: Type) => {
    type = t;
    part = new Part({
      owner: $user && $user.id,
      what: type.id,
      count: 0,
      climb: 0,
      descend: 0,
      distance: 0,
      time: 0,
      name: "",
      vendor: "",
      model: "",
      purchase: new Date(),
      last_used: new Date(),
    });
    isOpen = true;
  };

  const toggle = () => (isOpen = false);
  const setPart = (e: any) => {
    newpart = e.detail;
    disabled = false;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>New {type.name}</ModalHeader>
  <form on:submit|preventDefault={savePart}>
    <ModalBody>
      <NewForm {type} {part} on:change={setPart} />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Create"} />
    </ModalFooter>
  </form>
</Modal>
