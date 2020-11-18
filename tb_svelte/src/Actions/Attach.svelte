<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from 'sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, updatePartAttach, types} from '../store';
  import type {Attachment, Part} from '../types';  
  import AttachForm from './AttachForm.svelte';

  let attach: Attachment;
  let part: Part;
  let isOpen = false;
  let disabled = true;
  const toggle = () => isOpen = false

  async function action () {
    disabled = true;
    await myfetch('/attach/', 'PATCH', attach)
      .then(data => updatePartAttach(data))
    isOpen = false;  
  }  
  
  export const popup = (p: Part) => {
    part = p;
    isOpen = true
  };  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    Attach {$types[part.what].name} {part.name} {part.vendor} {part.model}
  </ModalHeader>
  <ModalBody>
    <AttachForm bind:attach bind:disabled {part} />
  </ModalBody>
  <ModalFooter {toggle} {action} {disabled} button={'Attach'} />
</Modal>