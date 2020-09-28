<script>
  import {
    Button,
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
    Spinner
  } from 'sveltestrap';
  import {handleError, myfetch, parts, updatePartAttach} from "./store.js";

  let promise;
  let isOpen = false;
  let files;
  export const toggle = () => {isOpen = !isOpen; files=undefined; promise = undefined};
  export const close = (e) => {isOpen = false; files=undefined; promise = undefined; alert(e)};

  $: disabled = !(files && files[0])

  async function sendFile () {
    var body = await files[0].text();
    return fetch('/activ/descend', {
            method: 'POST',
            credentials: 'include',
            body
        })
		    .then(response => response.json())
        .then(parts.updateMap)
  };
</script>

<div>
  <Modal {isOpen} {toggle}>
    <ModalHeader {toggle}>Upload Garmin activities file</ModalHeader>
    <ModalBody>      
      <input type="file" bind:files accept="text/csv">
    </ModalBody>
    <ModalFooter>
      <Button color="primary" {disabled} on:click={() => (promise = sendFile())}>
      {#await promise}
        <Spinner />
      {:then} 
        Synchronize
      <!-- {:catch error}
        {close(error)} -->
      {/await}
      </Button>
      <Button color="secondary" on:click={toggle}>Cancel</Button>
    </ModalFooter>
  </Modal>
</div>

