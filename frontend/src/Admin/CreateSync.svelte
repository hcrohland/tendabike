<script lang="ts">
  import {
    Modal,
    ModalHeader,
    ModalBody,
    Form,
    FormGroup,
    InputGroup,
    InputGroupText,
  } from "@sveltestrap/sveltestrap";
  import ModalFooter from "../Actions/ModalFooter.svelte";
  import { handleError, myfetch } from "../lib/store";
  import type { User } from "../lib/types";
  import DateTime from "../Widgets/DateTime.svelte";

  export let refresh: () => void;
  let user: User;
  let date = new Date();
  let isOpen = false;
  let userParam: string;
  const toggle = () => (isOpen = false);

  async function action() {
    await myfetch(
      "/strava/sync?time=" + (date.getTime() / 1000).toFixed(0) + userParam
    ).catch(handleError);
    isOpen = false;
    refresh();
  }

  export const createSync = (id?: User) => {
    user = id;
    if (id) {
      userParam = "&user_id=" + user.id;
    } else {
      userParam = "";
    }
    isOpen = true;
  };
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}>
    Create sync Event
    {#if user}
      for {user.firstname} {user.name} ({user.id})
    {/if}
  </ModalHeader>
  <ModalBody>
    <Form>
      <FormGroup check>
        <InputGroup>
          <InputGroupText>Start</InputGroupText>
          <DateTime maxdate={new Date()} bind:date />
        </InputGroup>
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {action} button={"Sync"} />
</Modal>
