<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from "@sveltestrap/sveltestrap";
  import MyFooter from "../Widgets/MyFooter.svelte";
  import { fmtDate } from "../lib/store";
  import { Part } from "../lib/part";
  import { types } from "../lib/types";

  let part: Part;
  let isOpen = false;
  const toggle = () => (isOpen = false);

  async function action() {
    part.disposed_at = undefined;
    await part.update();
    isOpen = false;
  }

  export const recoverPart = (p: Part) => {
    part = p;
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    Do you really have {types[part.what].name}
    {part.name}
    {part.vendor}
    {part.model} back?
  </ModalHeader>
  <ModalBody>
    You binned it on {fmtDate(part.disposed_at)}
  </ModalBody>
  <MyFooter {toggle} {action} label={"Recover"} />
</Modal>
