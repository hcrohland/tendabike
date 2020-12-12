<script lang="ts" >
import { Button, ButtonGroup, Spinner, Table } from 'sveltestrap';
import {handleError, myfetch} from '../store'
import type {User} from '../types'
import Sync from './Sync.svelte';
import CreateSync from "./CreateSync.svelte"

let promise, createSync;
let request:Promise<User[]> = myfetch('/user/all')
      .catch(handleError)

async function rescan() {
      promise = myfetch('/activ/rescan/').catch(handleError)
}

</script>
{#await request}
...
{:then userlist}
  <Table>
    <tr>
      <th>Id</th>
      <th>Name</th>
      <th>Role</th>
      <th></th>
    </tr>
    {#each userlist as user (user.id)}
    <tr>
      <td> {user.id}</td>
      <td> {user.firstname} {user.name} </td>
      <td> {user.is_admin ? "Admin" : "User"}</td>
      <td>
      <ButtonGroup>
        <Button on:click={() => createSync(user)}> Add Sync Event</Button>
        <Sync {user} />
      </ButtonGroup>
    </td>
  </tr>
  {/each}
</Table>
<ButtonGroup>
  <Button on:click={createSync}> Add Sync Event for all</Button>
  <Button on:click={rescan}> 
  {#await promise}
    <Spinner />
  {:then value}
    Rescan all activities
  {/await}
  </Button>
</ButtonGroup>

{/await}
<CreateSync bind:createSync />