<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    Form
  } from '@sveltestrap/sveltestrap';
  import {type AttEvent, Part, type Type} from '../lib/types';
  import {myfetch, handleError, user, updateSummary, attachments, filterValues} from '../lib/store';
  import ModalFooter from './ModalFooter.svelte'
  import NewForm from './NewForm.svelte';
  import TypeForm from './TypeForm.svelte';

  let part: Part, newpart: Part;
  let gear: Part;
  let type: Type;
  let hook: number;
  let disabled = true;
  let isOpen = false;
  const toggle = () => isOpen = false
  export const installPart = (g: Part) => {
    gear = g;
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
    disabled = true;
    type = undefined;
    isOpen = true
  }

  async function attachPart (part) {
    let attach: AttEvent = {
      part_id: part.id,
      time: part.purchase,
      gear: gear.id,
      hook
    }
    await myfetch('/part/attach', 'POST', attach)
        .then(updateSummary)
        .catch(handleError)
  }

  async function action () {
    disabled = true;
    await myfetch('/part', 'POST', newpart)
      .then(attachPart)
      .catch(handleError)
    isOpen = false;
  }

  function guessDate(g: Part, t: Type, hook: number) {
    if (!t) return new Date();
    let last = filterValues($attachments, (a) => a.gear == g.id && a.what == t.id && a.hook == hook)
    if (last.length) {
      //I t is a replacement
      return new Date()
    } else {
      // It is the first part of that type
      return g.purchase
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

  const setType = (e) => {
    type = e.detail.type;
    hook = e.detail.hook;
    part.what = type.id;
    part.purchase = guessDate(gear, type, hook)
  }
  const setPart = (e) => {
    newpart = e.detail
    disabled = false
  }

</script>
<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>  
    <TypeForm {gear} on:change={setType}/>
  </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} mindate={gear.purchase} on:change={setPart}/>
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Install'} />
</Modal>