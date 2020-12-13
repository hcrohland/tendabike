<script lang="ts">
  import {
    Form,
    Modal,
    ModalHeader,
    ModalBody
  } from 'sveltestrap';
  import type {Attachment, Part, Type} from '../types';
  import {myfetch, handleError, types, initData, parts, user, updateSummary} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import NewForm from './NewForm.svelte';
  import Dispose from '../Widgets/Dispose.svelte';

  let part: Part, oldpart: Part, newpart: Part;
  let type: Type;
  let prefix: string;
  let att: Attachment;
  let disabled = true;
  let dispose = false;
  let isOpen = false;
  const toggle = () => isOpen = false

  async function attachPart (part) {
    att.part_id = part.id;
    att.attached = part.purchase;
    att.detached = null
    await myfetch('/attach/', 'PATCH', att)
      .then(updateSummary)
      .catch(handleError)
    
    if (dispose) {
      oldpart.disposed_at = part.purchase;
      await myfetch('/part', 'PUT', oldpart)
        .then((data) => parts.updateMap([data]))
        .catch(handleError)
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
      purchase: att.detached || new Date(),
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
  <ModalHeader {toggle}>  New {prefix} {type.name} for {$parts[att.gear].name} </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} mindate={att.attached} on:change={setPart}/>
      <Dispose bind:dispose> old {type.name} </Dispose>
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Replace'} />
</Modal>