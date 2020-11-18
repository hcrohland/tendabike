<script lang="ts">
  import {
    Form,
    FormGroup,
    Modal,
    ModalHeader,
    ModalBody
  } from 'sveltestrap';
  import DateTime from './DateTime.svelte';
  import type {Attachment, Part, Type} from '../types';
  import {myfetch, types, initData, parts, user, updatePartAttach} from '../store';
  import ModalFooter from './ModalFooter.svelte'

  let part: Part;
  let type: Type;
  let prefix: string;
  let att: Attachment;
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

  $: disabled = !(part && part.name.length > 0 && part.vendor.length > 0 && part.model.length > 0)
  
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>  New {prefix} {type.name} for {$parts[att.gear].name} </ModalHeader>
  <ModalBody>
    <Form>
      <FormGroup row>
        <FormGroup class="col-md-12">
          <label for="inputName">You call it</label>
          <!-- svelte-ignore a11y-autofocus -->
          <input type="text" class="form-control" id="inputName" bind:value={part.name} autofocus required placeholder="Name">
        </FormGroup>
      </FormGroup>
      <FormGroup row>
        <FormGroup class="col-md-6">
          <label for="inputBrand">and it is a</label>
          <input type="text" class="form-control" id="inputBrand" bind:value={part.vendor} placeholder="Brand">
        </FormGroup>
        <FormGroup class="col-md-6">
          <label class="d-none d-md-block" for="inputModel"> &nbsp </label>
          <input type="text" class="form-control" id="inputModel" bind:value={part.model} placeholder="Model">
        </FormGroup>
      </FormGroup>
      <FormGroup row>
        <FormGroup class="col-md-6">
          <label for="inputDate">New {type.name} day was at</label>
          <DateTime id="inputDate" class="input-group-text" bind:date={part.purchase} mindate={att.attached} required/>
        </FormGroup>
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Replace'} />
</Modal>