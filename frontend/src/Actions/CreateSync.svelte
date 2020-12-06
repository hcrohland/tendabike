<script lang="ts">
  import { Modal, ModalHeader, ModalBody, 
          Form, FormGroup, Label, 
          Input, InputGroup, InputGroupAddon, InputGroupText, CustomInput
        } from 'sveltestrap';
  import ModalFooter from './ModalFooter.svelte'
  import {myfetch, updatePartAttach, types} from '../store';
  import DateTime from '../Widgets/DateTime.svelte'

  let user: string;
  let one_user = false;
  let date = new Date;
  let isOpen = false;
  let disabled = true;
  let placeholder: string;
  let userParam: string;
  const toggle = () => isOpen = false

  async function action () {
    disabled = true;
    await myfetch('/strava/sync?time=' + date.getTime()/1000 + userParam)
    isOpen = false;  
  }  
  
  export const createSync = () => {
    isOpen = true
  };  

  $: if (all_user) {
    disabled = false
    user = ''
    placeholder = "For all users"
    userParam = ''
  } else {
    disabled = user == '' ? true : false
    placeholder = "Input strava athlete id"
    userParam = '&user=' + user
  }

  $: all_user = !one_user
</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    Create sync Event
  </ModalHeader>
  <ModalBody>
    <Form>
      <FormGroup check>
        <InputGroup>
          <InputGroupAddon addonType="prepend">
            <InputGroupText>Start</InputGroupText>
          </InputGroupAddon>
          <DateTime maxdate={new Date} bind:date/> 
        </InputGroup>
        <br>
        <InputGroup>
          <InputGroupAddon addonType="prepend">
            <InputGroupText>
              <CustomInput
              type="switch"
              id="custominputneedsone"
              name="customSwitch"
              label="Select single user" bind:checked={one_user}/>
            </InputGroupText>
          </InputGroupAddon>

          <Input 
              {placeholder} disabled={all_user} type="number" bind:value={user}/>
        </InputGroup>
      </FormGroup>
    </Form>
  </ModalBody>
  <ModalFooter {toggle} {action} {disabled} button={'Attach'} />
</Modal>