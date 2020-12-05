<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    {dispose ? "Dispose" : "Detach"} {$types[part.what].name} 
    <br>{part.name} {part.vendor} {part.model} 
    <br>which was attached on {fmtDate(attach.attached)}
    to {$parts[attach.gear].name}
  </ModalHeader>
  <ModalBody>
    <Container>
      <Row >
        {#if last}
        <Dispose bind:dispose/>
        {/if}
        <DateTime bind:date={attach.detached} mindate={attach.attached}/> 
      </Row>
    </Container>
  </ModalBody>
  <ModalFooter {toggle} {action} button={'Detach'} />
</Modal>

<script lang="ts">
  import { 
      Modal, ModalHeader, ModalBody, Container, Row } from 'sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, updatePartAttach, types, parts, filterValues, attachments, fmtDate} from '../store';
  import type {Attachment, Part} from '../types';
  import DateTime from './DateTime.svelte';
  import Dispose from './Dispose.svelte';

  let attach: Attachment;
  let part: Part;
  let isOpen = false;
  let last, dispose;
  const toggle = () => isOpen = false

  async function action () {
    await myfetch('/attach/', 'PATCH', attach)
      .then(data => updatePartAttach(data))
    if (dispose) {
      part.disposed_at = attach.detached
      await myfetch('/part/', 'PUT', part)
        .then(data => updatePartAttach(data))
    }
    isOpen = false;  
  }  
  
  export const detachPart = (a: Attachment) => {
    attach = a;
    part = $parts[attach.part_id]
    isOpen = true
    last = filterValues(
        $attachments, 
        (a) => a.gear == attach.gear && a.hook == attach.hook && a.what == attach.what && a.attached > attach.attached
      ).length == 0
  };

</script>