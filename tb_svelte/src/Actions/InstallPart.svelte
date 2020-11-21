<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody
  } from 'sveltestrap';
  import type {Attachment, Part, Type} from '../types';
  import {myfetch, initData, user, updatePartAttach, attachments, filterValues} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import NewForm from './NewForm.svelte';
  import TypeForm from './TypeForm.svelte';

  let part: Part;
  let gear: Part;
  let type: Type;
  let attach: Attachment;
  let disabled = true;
  let isOpen = false;
  const toggle = () => isOpen = false
  export const popup = (g: Part) => {
    gear = g;
    attach = {
      part_id: undefined,
      gear: g.id,
      attached: new Date,
    }
    part = {
      owner: $user.id, 
      what: undefined, 
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: '', 
      vendor: '', 
      model: '', 
      purchase: new Date(),
      last_used: new Date()
    };
    type = undefined;
    isOpen = true
  }

  async function attachPart (part) {
    attach.part_id = part.id;
    attach.attached = part.purchase;
    await myfetch('/attach/', 'PATCH', attach)
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

  function guessDate(g: Part, t: Type, hook: number) {
    let last = filterValues<Attachment>($attachments, (a) => a.gear == g.id && a.what == t.id && a.hook == hook)
    if (last.length) {
      //I t is a replacement
      return new Date()
    } else {
      // It is the first part of that type
      return new Date(g.purchase)
    }
  }

  part = {
      owner: $user.id, 
      what: undefined, 
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: '', 
      vendor: '', 
      model: '', 
      purchase: new Date(),
      last_used: new Date()
    };


  $: if (type && attach.hook) {
    part.what = type && type.id;
    part.purchase = guessDate(gear, type, attach.hook)
    part.last_used = part.purchase
  }
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>  
    <TypeForm bind:type bind:attach {gear}/>
  </ModalHeader>
  <ModalBody>
    <NewForm bind:part bind:disabled {type}/>
  </ModalBody>
  <ModalFooter {action} {toggle} disabled={disabled || attach.hook == undefined} button={'Install'} />
</Modal>