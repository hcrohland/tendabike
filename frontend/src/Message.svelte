<script lang="ts">
  import { message } from "./lib/store";
  import { Modal, ModalBody, ModalHeader } from "flowbite-svelte";

  function toggle() {
    let m = $message;
    m.active = false;
    message.set(m);
  }
</script>

<Modal isOpen={$message.active}>
  <ModalHeader {toggle}>
    {$message.status}
  </ModalHeader>
  <ModalBody>
    {@html $message.message}
  </ModalBody>
</Modal>
