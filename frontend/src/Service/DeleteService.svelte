<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Actions/ModalFooter.svelte";
  import { Service } from "./service";
  import { fmtDate } from "../lib/store";

  let service: Service;
  let isOpen = false;
  const toggle = () => (isOpen = false);

  async function action() {
    await service.delete();
    isOpen = false;
  }

  export const deleteService = (s: Service) => {
    service = s;
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Do you really want to delete Service log<br />
    "{service.name}" from
    {fmtDate(service.time)}?
  </ModalHeader>
  <ModalFooter {toggle} {action} button={"Delete"} />
</Modal>
