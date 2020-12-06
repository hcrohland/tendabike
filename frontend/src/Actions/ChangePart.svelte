<script lang="ts">
  import {
    Modal, ModalBody, ModalHeader,
    FormGroup, InputGroup, Form
  } from 'sveltestrap';
  import {myfetch, parts, types, updatePartAttach, attachments, filterValues, by} from '../store';
  import ModalFooter from './ModalFooter.svelte'
  import type {Type, Part, Attachment} from '../types'
  import NewForm from './NewForm.svelte';
  import Dispose from './Dispose.svelte';
  import DateTime from './DateTime.svelte';
  import Switch from './Switch.svelte';
  
  let atts: Attachment[]
  let last: Attachment, start;
  let part: Part, newpart: Part;
  let type: Type = $types[1];
  let isGear = false
  let isOpen = false;
  let disabled = true, detach, part_changed;
  let dispose = false, date;

  async function savePart () {
    disabled = true;
    if (dispose) newpart.disposed_at = date
    if (detach) {
      last.detached = date
      await myfetch('/attach/', 'PATCH', last)
        .then(data => updatePartAttach(data))
    }
    if (dispose || part_changed){
      await myfetch('/part/', 'PUT', newpart)
        .then(data => parts.updateMap([data]))
    }

    isOpen = false;
  }

  export const changePart = (p: Part) => {
    part = p;
    newpart = p;
    type = $types[part.what];
    atts = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))
    start = atts[0] ? atts[0].attached : undefined;
    last = atts[atts.length - 1];
    date = undefined
    detach = false
    dispose = false
    part_changed = false
    isOpen = true;
    isGear = (part.what == type.main)
  }

  const toggle = () => isOpen = false
  const setPart = (e) => {
    newpart = e.detail
    part_changed = true;
  }

  $: disabled = (detach || dispose || part_changed) ? false : true 

</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> Change {type.name} details </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} on:change={setPart} maxdate={start}/>
      <FormGroup>
        {#if isGear}
          <InputGroup>
            <Dispose bind:dispose/>
            {#if dispose}
            <DateTime bind:date mindate={part.purchase}/>
            {/if}
          </InputGroup>
        {:else if last}
          {#if last.detached}
          <InputGroup>
            <Dispose bind:dispose/>
            {#if dispose}
            <DateTime bind:date mindate={last.detached}/>
            {/if}
          </InputGroup>
          {:else}
          <InputGroup>
            <Switch bind:checked={detach}>
              {#if detach}
              detached at
              {:else}
              detach?
              {/if}
            </Switch>
            {#if detach}
               <DateTime bind:date mindate={last.attached}/>
               <Dispose bind:dispose>  {type.name} when detached </Dispose>
            {/if}
          </InputGroup>
          {/if}
        {/if}
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {disabled} action={savePart} button={'Change'}/>
</Modal>