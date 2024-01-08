<script lang="ts">
  import { 
    Modal, ModalHeader, ModalBody,
  } from '@sveltestrap/sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, handleError, types, fmtDate, parts} from '../lib/store';
  import type {Part} from '../lib/types';  

  let part: Part;
  let isOpen = false;
  const toggle = () => isOpen = false

  async function action () {
    part.disposed_at = null;
    await myfetch('/part', 'PUT', part)
      .then(data => parts.updateMap([data]))
      .catch(handleError)
    isOpen = false;  
  }  
  
  export const recoverPart = (p: Part) => {
    part = p;
    isOpen = true
  };  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    Do you really have {types[part.what].name} {part.name} {part.vendor} {part.model} back?
  </ModalHeader>
  <ModalBody>
    You binned it on {fmtDate(part.disposed_at)}
  </ModalBody>
  <ModalFooter {toggle} {action} button={'Recover'} />
</Modal>