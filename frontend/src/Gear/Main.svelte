<script lang="ts">
  import {DropdownItem, Table} from 'sveltestrap'
  import {filterValues, types, parts, attachments, category, fmtDate} from '../store'
  import Subparts from './Subparts.svelte'
  import Usage from '../Usage.svelte'
  import type {Attachment, Part} from '../types'
  import InstallPart from '../Actions/InstallPart.svelte'
  import ChangePart from '../Actions/ChangePart.svelte'
  import Menu from '../Menu.svelte'
 
  export let params;
  
  let hook, gear: Part;
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

<Table>
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
      <td>
        {fmtDate(gear.purchase)}
        {#if gear.disposed_at}
           - {fmtDate(gear.disposed_at)}
        {/if}
      </td>
      <Usage part={gear} />
      <td>
        <Menu>
          <DropdownItem on:click={() => installPart(gear)}> Attach new part </DropdownItem>
          <DropdownItem on:click={() => changePart(gear)}> Change details </DropdownItem>
        </Menu>
      </td>
    </tr>
  </tbody>
</Table>
<Subparts {hook} {attachees} />

<InstallPart bind:installPart />
<ChangePart bind:changePart />