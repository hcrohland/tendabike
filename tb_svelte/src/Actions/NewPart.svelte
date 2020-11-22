<script lang="ts">
  import {
    Modal,
    ModalBody,
    ModalHeader,
  } from 'sveltestrap';
  import {myfetch, initData, parts, user} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import type {Type, Part} from '../types'
  import NewForm from './NewForm.svelte';

  let part: Part;
  let type: Type;
  let isOpen = false;
  let disabled = true;

  async function savePart () {
    disabled = true;
    try {
      await myfetch('/part/', 'POST', part)
        .then(data => parts.updateMap([data]))
    } catch (e) {
      alert (e)
      initData()
    }
    isOpen = false;
  }

  export const popup = (t: Type) => {
    type = t;
    part = {
      owner: $user.id,
      what: type.id,
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: "",
      vendor: "",
      model: "",
      purchase: new Date(),
      last_used: new Date()
    };
    isOpen = true;
  }

  const toggle = () => isOpen = false
  const setPart = (e) => {
    part.name = e.detail.name
    part.vendor = e.detail.vendor
    part.model = e.detail.model
    part.purchase = e.detail.purchase
    disabled = false
  }
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> New {type.name} </ModalHeader>
  <ModalBody>
    <NewForm {type} {part} on:change={setPart}/>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={'Attach'}/>
</Modal>