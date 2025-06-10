<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
  } from "@sveltestrap/sveltestrap";
  import { Service } from "../lib/service";
  import ServiceForm from "./ServiceForm.svelte";
  import { parts, Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";

  let part: Part;
  let service: Service, newservice: Service;
  let isOpen = false;
  let disabled = true;

  async function saveService() {
    disabled = true;
    await newservice.redo();
    isOpen = false;
  }

  export const redoService = (s: Service) => {
    part = $parts[s.part_id];
    service = new Service(s);
    isOpen = true;
  };

  const toggle = () => (isOpen = false);

  const setService = (e: any) => {
    newservice = e.detail;
    disabled = false;
  };
</script>

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>
    Redo Service for {part.name}
    {part.vendor}
    {part.model}
  </ModalHeader>
  <form on:submit|preventDefault={saveService}>
    <ModalBody>
      <ServiceForm
        {service}
        mindate={part.purchase}
        noname
        on:change={setService}
      />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Save"} />
    </ModalFooter>
  </form>
</Modal>
