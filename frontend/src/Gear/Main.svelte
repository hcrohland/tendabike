<script lang="ts">
  import {DropdownItem} from 'sveltestrap'
  import {filterValues, types, parts, attachments, category} from '../store'
  import Subparts from './Subparts.svelte'
  import Usage from '../Usage.svelte'
  import type {Attachment} from '../types'
  import InstallPart from '../Actions/InstallPart.svelte'
  import ChangePart from '../Actions/ChangePart.svelte'
  import Menu from '../Menu.svelte'
 
  export let params;
  
  let hook, gear;
  let installPart, changePart;

  $: {
    gear = $parts[params.id]; 
    hook = $types[gear.what];
    category.set(hook)
  }
  $: attachees = filterValues(
    $attachments, 
    (a) => a.gear == gear.id
  ) as Attachment[]
</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>
<div class="scroll-x">
  <table class="table">
    <thead>
      <tr>
        <th scope="col">{hook.name}</th>
        <th scope="col">Brand</th>
        <th scope="col">Model</th>
        <th scope="col">Purchase</th>
        <Usage header/>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{gear.name}</td>
        <td>{gear.vendor}</td>
        <td>{gear.model}</td>
        <td>{new Date(gear.purchase).toLocaleDateString(navigator.language)}</td>
        <Usage part={gear} />
        <td>
          <Menu>
            <DropdownItem on:click={() => changePart(gear)}> Change details </DropdownItem>
            <DropdownItem on:click={() => installPart(gear)}> Attach new part </DropdownItem>
          </Menu>
        </td>
      </tr>
    </tbody>
  </table>
  <Subparts {hook} {attachees} />
</div>
<InstallPart bind:popup={installPart} />
<ChangePart bind:popup={changePart} />