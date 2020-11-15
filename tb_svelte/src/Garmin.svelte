<script>
  import {
    Button,
    Modal,
    ModalBody,
    ModalFooter,
    ModalHeader,
    Spinner,
    ListGroup,
    ListGroupItem,
  } from 'sveltestrap';
  import {checkStatus, parts} from "./store";
  import TimezonePicker from 'svelte-timezone-picker';
 
  let timezone  ;

  let promise;
  let isOpen = false;
  let files;
  let result;
  export const toggle = () => {isOpen = !isOpen; reset()};
  export const close = (e) => {isOpen = false; reset(); alert(e)};

  const reset = () => {
    files=undefined; 
    promise = undefined; 
    timezone = undefined;
    result = undefined;
  }

  $: disabled = !(files && files[0])

  async function sendFile () {
    var body = await files[0].text();
    return fetch('/activ/descend?tz=' + timezone, {
            method: 'POST',
            credentials: 'include',
            body
        })
        .then(checkStatus)
        .then((a) => {
            parts.updateMap(a[0]); 
            result = {
              good: a[1],
              bad: a[2]
            }
          }
        )
  };
</script>

<div>
  <Modal {isOpen} {toggle}>
    <ModalHeader {toggle}>Upload Garmin activities file</ModalHeader>
    {#if result}
      <ModalBody>
        {#if result.good.length > 0}
          Synchronized {result.good.length} activities. 
        {/if}
        {#if result.bad.length > 0}
          <br><br>Could not match the following {result.bad.length} activities:
          <ListGroup>
            {#each result.bad as r}
            <ListGroupItem>{r}</ListGroupItem>
            {/each}
          </ListGroup>
        {/if}
      </ModalBody>
      <ModalFooter>
        <Button color="primary" on:click={toggle}>Close</Button>
      </ModalFooter>
    {:else}
       <ModalBody>      
         <input type="file" bind:files accept="text/csv">
         <br>
         <div class="container"> 
           Timezone of activities: <TimezonePicker bind:timezone />
          </div>
        </ModalBody>
        <ModalFooter>
          <Button color="primary" {disabled} on:click={() => (promise = sendFile())}>
            {#await promise}
            <Spinner />
            {:then} 
            Synchronize
            {:catch error}
            {close(error)}
            {/await}
          </Button>
          <Button color="secondary" on:click={toggle}>Cancel</Button>
        </ModalFooter>
    {/if}
  </Modal>
</div>

