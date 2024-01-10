<script lang="ts">
  import {
    Form,
    Modal,
    ModalHeader,
    ModalBody
  } from '@sveltestrap/sveltestrap';
  import {AttEvent, Attachment, Part, Type, maxDate} from '../lib/types';
  import {types, parts, user} from '../lib/store';
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
  let mindate: Date;
  const toggle = () => isOpen = false

  async function attachPart (part: Part | void) {
    if (!part) throw ("Replace: update part did fail")
    await new AttEvent(part.id, part.purchase, gear, hook).attach();
    
    if (dispose) {
      oldpart.disposed_at = part.purchase;
      await oldpart.update()
    }
  }

  async function action () {
    disabled = true;
    await newpart.create()
      .then(attachPart)
    isOpen = false;
    isOpen = false;
}

export const replacePart = (attl: Attachment) => {
    oldpart = $parts[attl.part_id];
    hook = attl.hook;
    gear= attl.gear;
    mindate = attl.attached;
    type = oldpart.type();
    prefix = types[attl.hook].prefix;
    part = new Part({
        owner: $user && $user.id, 
        what: oldpart.what, 
        name: oldpart.name, 
        vendor: oldpart.vendor, 
        model: oldpart.model, 
        purchase: attl.detached < maxDate ? attl.detached : new Date(),
      });
    dispose = false;
    isOpen = true;
  }

  const setPart = (e: CustomEvent<Part>) => {
    newpart = new Part(e.detail);
    disabled = false
  }
  
</script>

<Modal {isOpen} {toggle} backdrop={false}>
  <ModalHeader {toggle}>  New {prefix} {type.name} for {$parts[gear].name} </ModalHeader>
  <ModalBody>
    <Form>
      <NewForm {type} {part} {mindate} on:change={setPart}/>
      <Dispose bind:dispose> old {type.name} </Dispose>
    </Form>
  </ModalBody>
  <ModalFooter {action} {toggle} {disabled} button={'Replace'} />
</Modal>