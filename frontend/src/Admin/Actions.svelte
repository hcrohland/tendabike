<script lang="ts" >
import { Button, ButtonGroup } from 'sveltestrap';
import {handleError, myfetch} from '../store'
import type {User} from '../types'

export let user: User;

let promise, count

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

<ButtonGroup>
  <Button on:click={() => sync(user.id)}> 
  {#await promise}
    Syncing {count} ...
  {:then}
    Sync
  {/await} </Button>
  <!-- <Button> Disable</Button> -->
</ButtonGroup>