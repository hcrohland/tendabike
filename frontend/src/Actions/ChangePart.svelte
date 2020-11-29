<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
  } from 'sveltestrap';
  import {myfetch, initData, parts, types} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import type {Type, Part} from '../types'
  import NewForm from './NewForm.svelte';

  let part, newpart: Part;
  let type: Type;
  let isOpen = false;
  let disabled = true;

  async function savePart () {
    disabled = true;
    try {
      await myfetch('/part/', 'PUT', newpart)
        .then(data => parts.updateMap([data]))
    } catch (e) {
      alert (e)
      initData()
    }
    isOpen = false;
  }

  export const popup = (p: Part) => {
    part = p;
    type = $types[part.what];
    isOpen = true;
  }

  const toggle = () => isOpen = false
  const setPart = (e) => {
    newpart = e.detail
    disabled = false
  }
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> Change {type.name} details </ModalHeader>
  <ModalBody>
    <NewForm {type} {part} on:change={setPart}/>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={'Change'}/>
</Modal>