<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    Form,
  } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { Service } from "./service";
  import ServiceForm from "./ServiceForm.svelte";
  import { parts, Part } from "../lib/part";

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

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Redo Service for {part.name}
    {part.vendor}
    {part.model}
  </ModalHeader>
  <ModalBody>
    <Form>
      <ServiceForm
        {service}
        mindate={part.purchase}
        noname
        on:change={setService}
      />
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={saveService} button={"Save"} />
</Modal>
