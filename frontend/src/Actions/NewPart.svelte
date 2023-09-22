<script lang="ts">
  import {
    Modal, ModalBody, ModalHeader,
    Form
  } from 'sveltestrap';
  import NewForm from './NewForm.svelte';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, handleError, initData, parts, user} from '../store';
  import type {Type, Part} from '../types'

  let part, newpart: Part;
  let type: Type;
  let isOpen = false;
  let disabled = true;

  async function savePart () {
    disabled = true;
    await myfetch('/part', 'POST', newpart)
      .then(data => parts.updateMap([data]))
      .catch(handleError)
    isOpen = false;
  }

  export const newPart = (t: Type) => {
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
    newpart = e.detail
    disabled = false
  }
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> New {type.name} </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} on:change={setPart}/>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={'Attach'}/>
</Modal>