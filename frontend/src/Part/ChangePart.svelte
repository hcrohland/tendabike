<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
    FormGroup,
    InputGroup,
    Form,
  } from "@sveltestrap/sveltestrap";
  import { handleError, types } from "../lib/store";
  import ModalFooter from "../Widgets/ModalFooter.svelte";
  import { maxDate } from "../lib/types";
  import { AttEvent, Attachment, attachments } from "../Attachment/attachment";
  import NewForm from "./PartForm.svelte";
  import Dispose from "../Widgets/Dispose.svelte";
  import DateTime from "../Widgets/DateTime.svelte";
  import Switch from "../Widgets/Switch.svelte";
  import { Part } from "../lib/part";

  let atts: Attachment[];
  let last: Attachment;
  let start: Date | undefined;
  let part: Part;
  let newpart: Part;
  let type = types[1]; // will be set later
  let isGear = false;
  let isOpen = false;
  let disabled = true,
    detach: boolean,
    part_changed: boolean,
    dispose = false;
  let date: Date;

  async function savePart() {
    try {
      disabled = true;
      if (dispose) newpart.disposed_at = date;
      if (detach) {
        await new AttEvent(last.part_id, date, last.gear, last.hook).detach();
      }
      if (dispose || part_changed) {
        await newpart.update();
      }
    } catch (e: any) {
      handleError(e);
    }

    isOpen = false;
  }

  export const changePart = (p: Part) => {
    part = p;
    newpart = p;
    type = part.type();
    atts = part.attachments($attachments);
    last = atts[0];
    start = atts.length > 0 ? atts[atts.length - 1].attached : undefined;
    date = last && last.detached < maxDate ? last.detached : new Date();
    detach = false;
    dispose = false;
    part_changed = false;
    isOpen = true;
    isGear = part.what == type.main;
  };

  const toggle = () => (isOpen = false);
  const setPart = (e: CustomEvent<Part>) => {
    newpart = new Part(e.detail);
    part_changed = true;
  };

  $: disabled = !(detach || dispose || part_changed);
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>Change {type.name} details</ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} on:change={setPart} maxdate={start} />
      <FormGroup>
        {#if isGear}
          <InputGroup>
            <Dispose bind:dispose />
            {#if dispose}
              <DateTime bind:date mindate={part.purchase} />
            {/if}
          </InputGroup>
        {:else if last}
          {#if last.detached < maxDate}
            <InputGroup>
              <Dispose bind:dispose />
              {#if dispose}
                <DateTime bind:date mindate={last.detached} />
              {/if}
            </InputGroup>
          {:else}
            <InputGroup>
              <Switch bind:checked={detach}>
                {#if detach}
                  detached
                {:else}
                  detach?
                {/if}
              </Switch>
              {#if detach}
                <DateTime bind:date mindate={last.attached} />
                <Dispose bind:dispose>{type.name} when detached</Dispose>
              {/if}
            </InputGroup>
          {/if}
        {/if}
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={"Change"} />
</Modal>
