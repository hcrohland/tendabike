<script lang="ts">
  import {
    Form,
    FormGroup,
    Modal,
    ModalHeader,
    ModalBody
  } from 'sveltestrap';
  import type {Attachment, Part, Type} from '../types';
  import {myfetch, types, initData, parts, user, updatePartAttach} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import NewForm from './NewForm.svelte';

  let part: Part;
  let type: Type;
  let prefix: string;
  let att: Attachment;
  let disabled = true;
  let isOpen = false;
  const toggle = () => isOpen = false

  async function attachPart (part) {
    att.part_id = part.id;
    att.attached = part.purchase;
      await myfetch('/attach/', 'PATCH', att)
        .then(data => updatePartAttach(data))
  }

  async function action () {
    disabled = true;
    try {
      await myfetch('/part/', 'POST', part)
        .then(attachPart)
      isOpen = false;
    } catch (e) {
      alert (e)
      initData()
    }
    isOpen = false;
}

export const popup = (attl: Attachment) => {
    let oldpart = $parts[attl.part_id];
    att = {...attl};
    type = $types[oldpart.what];
    prefix = $types[attl.hook].name.split(' ').reverse()[1] || '' // The first word iff there were two (hack!)
    part = {
      owner: $user.id, 
      what: oldpart.what, 
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: oldpart.name, 
      vendor: oldpart.vendor, 
      model: oldpart.model, 
      purchase: new Date(),
      last_used: new Date()
    };
    isOpen = true;
  }

  const setPart = (e) => {
    part.name = e.detail.name
    part.vendor = e.detail.vendor
    part.model = e.detail.model
    part.purchase = e.detail.purchase
    part.last_used = part.purchase
    disabled = false
  }
  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>  New {prefix} {type.name} for {$parts[att.gear].name} </ModalHeader>
  <ModalBody>
    <NewForm {type} {part} on:change={setPart}/>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Replace'} />
</Modal>