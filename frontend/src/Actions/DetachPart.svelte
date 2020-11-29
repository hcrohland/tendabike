<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    Detach {$types[part.what].name} {part.name} {part.vendor} {part.model} <br>
    which was attached at {new Date(attach.attached).toLocaleDateString(navigator.language)}
    to {$parts[attach.gear].name}
  </ModalHeader>
  <ModalBody>
    <DateTime class="input-group-text" bind:date={attach.detached} mindate={attach.attached}/> 
  </ModalBody>
  <ModalFooter {toggle} {action} button={'Detach'} />
</Modal>

<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from 'sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, updatePartAttach, types, parts} from '../store';
  import type {Attachment, Part} from '../types';  
  import DateTime from './DateTime.svelte';

  let attach: Attachment;
  let part: Part;
  let isOpen = false;
  const toggle = () => isOpen = false

  async function action () {
    await myfetch('/attach/', 'PATCH', attach)
      .then(data => updatePartAttach(data))
    isOpen = false;  
  }  
  
  export const detachPart = (a: Attachment) => {
    attach = a;
    part = $parts[attach.part_id]
    isOpen = true
  };
</script>