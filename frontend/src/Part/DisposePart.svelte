<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    FormGroup,
    InputGroup,
    ModalFooter,
    InputGroupText,
  } from "@sveltestrap/sveltestrap";
  import { handleError } from "../lib/store";
  import { Attachment } from "../lib/attachment";
  import Dispose from "../Widgets/Dispose.svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import { Part } from "../lib/part";
  import Buttons from "../Widgets/Buttons.svelte";
  import Switch from "../Widgets/Switch.svelte";

  let isOpen = false;
  let disabled = true;
  let last: Attachment | undefined;
  let part: Part;
  let name: String;
  let detach: boolean;
  let dispose: boolean;
  let mindate: Date;
  let date: Date;
  let all: boolean;

  async function savePart() {
    try {
      disabled = true;
      if (detach) {
        await part.detach(date, all);
      }
      if (dispose) {
        await part.dispose(date, all);
      }
    } catch (e: any) {
      handleError(e);
    }
    isOpen = false;
  }

  export const disposePart = (p: Part, last_attachment?: Attachment) => {
    part = p;
    name = part.type().name;
    last = last_attachment;

    if (last) {
      if (last.isDetached()) {
        detach = false;
        dispose = true;
        mindate = last.detached;
      } else {
        detach = true;
        dispose = false;
        mindate = last.attached;
      }
    } else {
      mindate = part.purchase;
      detach = false;
      dispose = true;
    }
    all = true;
    date = new Date();
    disabled = false;
    isOpen = true;
  };

  $: label = detach ? "Detach " : "Dispose ";
  const toggle = () => (isOpen = false);
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>
    {label}
    {name}
    {part.name}
  </ModalHeader>
  <form on:submit|preventDefault={savePart}>
    <ModalBody>
      <FormGroup>
        <InputGroup>
          <InputGroupText>At</InputGroupText>
          <DateTime bind:date {mindate} />
          <Switch bind:checked={all}>{label} all attached parts</Switch>
          {#if detach}
            <Dispose bind:dispose>{name} when detached</Dispose>
          {/if}
        </InputGroup>
      </FormGroup>
    </ModalBody>
    <ModalFooter>
      <Buttons {toggle} {disabled} label={detach ? "Detach" : "Dispose"} />
    </ModalFooter>
  </form>
</Modal>
