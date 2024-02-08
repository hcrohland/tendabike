<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    Form,
  } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Actions/ModalFooter.svelte";
  import { Service } from "./service";
  import { Part } from "../Part/part";
  import ServiceForm from "./ServiceForm.svelte";

  let part: Part;
  let service: Service, newservice: Service;
  let isOpen = false;
  let disabled = true;

  async function saveService() {
    disabled = true;
    await Service.create(
      newservice.part_id,
      newservice.time,
      newservice.name,
      newservice.notes,
    );
    isOpen = false;
  }

  export const newService = (p: Part) => {
    part = p;
    service = new Service({
      part_id: p.id,
    });
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
    New Service for {part.name}
    {part.vendor}
    {part.model}
  </ModalHeader>
  <ModalBody>
    <Form>
      <ServiceForm {service} mindate={part.purchase} on:change={setService} />
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={saveService} button={"Create"} />
</Modal>
