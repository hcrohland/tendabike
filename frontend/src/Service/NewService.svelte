<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    ModalFooter,
  } from "@sveltestrap/sveltestrap";
  import { Service } from "../lib/service";
  import { Part } from "../lib/part";
  import ServiceForm from "./ServiceForm.svelte";
  import Buttons from "../Widgets/Buttons.svelte";

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
      newservice.plans,
    );
    isOpen = false;
  }

  export const newService = (p: Part, plans: string[] = []) => {
    part = p;
    service = new Service({
      part_id: p.id,
      plans,
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
  <form on:submit|preventDefault={saveService}>
    <ModalBody>
      <ServiceForm {service} mindate={part.purchase} on:change={setService} />
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={"Save"} />
    </ModalFooter>
  </form>
</Modal>
