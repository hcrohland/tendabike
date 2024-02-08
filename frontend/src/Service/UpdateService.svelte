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
  import { parts, Part } from "../Part/part";

  let part: Part;
  let service: Service, newservice: Service;
  let isOpen = false;
  let disabled = true;

  async function saveService() {
    disabled = true;
    await newservice.update();
    isOpen = false;
  }

  export const updateService = (s: Service) => {
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
    Update Service for {part.name}
    {part.vendor}
    {part.model}
  </ModalHeader>
  <ModalBody>
    <Form>
      <ServiceForm
        {service}
        mindate={part.purchase}
        finish
        on:change={setService}
      />
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={saveService} button={"Update"} />
</Modal>
