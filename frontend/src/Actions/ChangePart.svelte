<script lang="ts">
  import {
    Modal, ModalBody, ModalHeader,
    FormGroup, InputGroup, Form
  } from 'sveltestrap';
  import {myfetch, initData, parts, types, attachments, filterValues} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import type {Type, Part, Attachment} from '../types'
  import NewForm from './NewForm.svelte';
  import Dispose from './Dispose.svelte';
  import DateTime from './DateTime.svelte';

  let part: Part, newpart: Part;
  let type: Type;
  let isOpen = false;
  let disabled = true;
  let dispose = false, date;

  async function savePart () {
    disabled = true;
    if (dispose) newpart.disposed_at = date
    try {
      await myfetch('/part/', 'PUT', newpart)
        .then(data => parts.updateMap([data]))
    } catch (e) {
      alert (e)
      initData()
    }
    isOpen = false;
  }

  export const changePart = (p: Part) => {
    part = p;
    newpart = p;
    if (dispose) part.disposed_at = date;
    type = $types[part.what];
    isOpen = true;
  }

  function isAttached(part:Part) {
    return filterValues<Attachment>($attachments, (a) => a.part_id == part.id && !a.detached).pop()
  }

  const toggle = () => isOpen = false
  const setPart = (e) => {
    newpart = e.detail
    disabled = false
  }

  $: if (dispose) disabled = false
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> Change {type.name} details </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} on:change={setPart}/>
      <FormGroup>
        {#if !isAttached(part)}
        <InputGroup>
          <Dispose bind:dispose/>
          {#if dispose}
          <DateTime bind:date />
          {/if}
        </InputGroup>
        {/if}
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={'Change'}/>
</Modal>