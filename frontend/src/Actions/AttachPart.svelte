<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from '@sveltestrap/sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, handleError, updateSummary, types} from '../store';
  import type {AttEvent, Part} from '../types';  
  import AttachForm from './AttachForm.svelte';

  let attach: AttEvent;
  let part: Part;
  let isOpen = false;
  let disabled = true;
  const toggle = () => {part= undefined; isOpen = false}

  async function action () {
    disabled = true;
    await myfetch('/part/attach', 'POST', attach)
      .then(updateSummary)
      .catch(handleError)
    isOpen = false;  
  }  
  
  export const attachPart = (p: Part) => {
    part = p;
    isOpen = true
  };  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    Attach {types[part.what].name} {part.name} {part.vendor} {part.model}
  </ModalHeader>
  <ModalBody>
    <AttachForm bind:attach bind:disabled {part} />
  </ModalBody>
  <ModalFooter {toggle} {action} {disabled} button={'Attach'} />
</Modal>