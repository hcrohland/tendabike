<script lang="ts">
  import {
    Form,
    Modal,
    ModalHeader,
    ModalBody
  } from 'sveltestrap';
  import type {AttEvent, Attachment, Part, Type} from '../types';
  import {myfetch, handleError, types, parts, user, updateSummary, maxDate} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import NewForm from './NewForm.svelte';
  import Dispose from '../Widgets/Dispose.svelte';

  let part: Part, oldpart: Part, newpart: Part;
  let type: Type;
  let prefix: string;
  let gear: number;
  let hook: number;
  let disabled = true;
  let dispose = false;
  let isOpen = false;
  let mindate;
  const toggle = () => isOpen = false

  async function attachPart (part) {
    let evt: AttEvent = {
      gear,
      hook,
      part_id: part.id,
      time: part.purchase,
    }
    await myfetch('/part/attach', 'POST', evt)
      .then(updateSummary)
    
    if (dispose) {
      oldpart.disposed_at = part.purchase;
      await myfetch('/part', 'PUT', oldpart)
        .then((data) => parts.updateMap([data]))
    }
  }

  async function action () {
    disabled = true;
    await myfetch('/part/', 'POST', newpart)
      .then(attachPart)
      .catch(handleError)
    isOpen = false;
    isOpen = false;
}

export const replacePart = (attl: Attachment) => {
    oldpart = $parts[attl.part_id];
    hook = attl.hook;
    gear= attl.gear;
    mindate = attl.attached;
    type = types[oldpart.what];
    prefix = types[attl.hook].prefix;
    part = {
      owner: $user.id, 
      what: oldpart.what, 
      count:0, climb:0, descend:0, distance:0, time: 0,
      name: oldpart.name, 
      vendor: oldpart.vendor, 
      model: oldpart.model, 
      purchase: attl.detached < maxDate ? attl.detached : new Date(),
      last_used: undefined
    };
    dispose = false;
    isOpen = true;
  }

  const setPart = (e) => {
    newpart = e.detail
    disabled = false
  }
  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>  New {prefix} {type.name} for {$parts[gear].name} </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} {mindate} on:change={setPart}/>
      <Dispose bind:dispose> old {type.name} </Dispose>
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Replace'} />
</Modal>