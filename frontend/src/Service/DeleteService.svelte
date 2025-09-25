<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "flowbite-svelte";
  import MyFooter from "../Widgets/MyFooter.svelte";
  import { Service } from "../lib/service";
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

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>
    Do you really want to delete Service log<br />
    "{service.name}" from
    {fmtDate(service.time)}?
  </ModalHeader>
  <MyFooter {toggle} {action} label={"Delete"} />
</Modal>
