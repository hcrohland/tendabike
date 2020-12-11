<script lang="ts" >
import { Button, ButtonGroup, Table } from 'sveltestrap';
import About from './About.svelte';
import {handleError, message, myfetch, user} from './store'

type User = {
  id: number,
  firstname: string,
  name: string,
  is_admin: boolean
}

let count, promise
let request:Promise<User[]> = myfetch('/user/all')
      .catch(handleError)

const sync = (id) => {promise = getdata(id)}
async function getdata(id) {
    let data
    count = 0;
    do {
      data = await myfetch('/strava/sync/' + id).catch(handleError)
      if (!data) break;
      count += data["activities"].length;
    } while (data["activities"].length > 0)
    count = 0;
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
           <Button on:click={() => sync(user.id)}> 
            {#await promise}
              Syncing {count} ...
            {:then}
              Sync
            {/await} </Button>
           <!-- <Button> Disable</Button> -->
         </ButtonGroup>
       </td>
      </tr>
      {/each}
    </Table>
    <!-- <ButtonGroup>
      <Button> Disable Webhook</Button>
      <Button> Stop Server</Button>
    </ButtonGroup> -->
{/await}