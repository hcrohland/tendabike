<script lang="ts">
  import { Modal, ModalHeader } from "@sveltestrap/sveltestrap";
  import MyFooter from "../Widgets/MyFooter.svelte";
  import { type Attachment } from "../lib/attachment";
  import { Part, parts } from "../lib/part";
  import { fmtDate } from "../lib/store";

  let attachment: Attachment;
  let part: Part;
  let isOpen = false;
  const toggle = () => (isOpen = false);

  async function action() {
    await part.detach(attachment.attached, true);
    isOpen = false;
  }

  export const deleteAttachment = (a: Attachment) => {
    attachment = a;
    part = $parts[a.part_id];
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle}>
  <ModalHeader {toggle}>
    Do you really want to remove part {part.name} from
    {$parts[attachment.gear].name} at {fmtDate(attachment.attached)}
  </ModalHeader>
  <MyFooter {toggle} {action} label={"Delete"} />
</Modal>
