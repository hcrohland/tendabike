<script lang="ts">
  import { Modal, ModalHeader, ModalBody } from 'sveltestrap';
  import DateTime from './DateTime.svelte';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, updatePartAttach, attachments, initData, filterValues, types, parts, by} from '../store';
  import type {Attachment, Type, Part} from '../types';  

  let attach: Attachment;
  let type: Type;
  let options: Part[];
  let isOpen = false;
  let header;
  let disabled = true;
  
  function lastDetach(part) {
    let last = filterValues<Attachment>($attachments, (a) => a.part_id == part.id).sort(by("attached"))[0]
    
    if (last) {
      return last.detached ? last.detached : last.attached
    } else {
      return part.purchase
    }
  }

  async function action () {
    disabled = true;
    await myfetch('/attach/', 'PATCH', attach)
      .then(data => updatePartAttach(data))
    isOpen = false;
  }
  
  export const popup = (part: Part) => {
    type = $types[part.what]; 
    attach = {
      part_id: part.id,
      attached: lastDetach(part),
      gear: undefined,
      hook: (type.hooks.length == 1) ? type.hooks[0] : undefined,
      detached: null,
    }   
    header = ["Attach",type.name,part.name,part.vendor,part.model].join(' ');
    options = filterValues<Part>($parts, (p) => type.main == p.what)
    isOpen = true
  };

  const toggle = () => isOpen = false

  $: disabled = attach && !($types[attach.hook] && $parts[attach.gear])
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>{header}</ModalHeader>
  <ModalBody>
    <form>
      <div class="form-inline">
        <div class="input-group mb-0 mr-sm-2 mb-sm-2">
          <div class="input-group-prepend">
            <span class="input-group-text">to</span>
          </div>
          {#if type.hooks.length > 1}
            <!-- svelte-ignore a11y-autofocus -->
            <select name="hook" class="form-control" autofocus required bind:value={attach.hook}>
              <option hidden value={undefined}> -- select one -- </option>
              {#each type.hooks as h}
              <option value={h}>{$types[h].name}</option>
              {/each}
            </select>
            <div class="input-group-prepend">
              <span class="input-group-text">of</span>
            </div>
          {/if}
          <select name="gear" class="form-control" required bind:value={attach.gear}>
            <option hidden value> -- select one -- </option>
            {#each options as gear}
              <option value={gear.id}>{gear.name}</option>
            {/each}
          </select> 
        </div>  
        <div class="input-group mb-0 mr-sm-2 mb-sm-2">
          <div class="input-group-prepend">
            <span class="input-group-text">at</span>
          </div>
          <DateTime class="input-group-text" bind:date={attach.attached}/> 
        </div>
      </div>
    </form> 
  </ModalBody>
  <ModalFooter {toggle} {action} {disabled} button={'Attach'}>
  </ModalFooter>
</Modal>