<script>
import {types} from './store.js'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let hook;
export let subparts;
export let level = undefined;

let part

</script>
{#if subparts.length > 0}
{#if level === undefined}
  <table class="table table-hover">
  <thead>
    <tr>
      <th scope="col">Part</th>
      <th scope="col">Name</th>
      <th scope="col" class="text-right">Attached</th>
      <Usage />
      <th></th>
    </tr>
  </thead>
  <tbody>
    <svelte:self {hook} {subparts} level={0}></svelte:self>
  </tbody>
  </table>
  
{:else}

  {#each subparts
      .sort((a,b) => $types[a.what].order > $types[b.what].order)
      .filter((a) => a.hook == hook)
    as {what, name, part_id, attached} (part_id)}
    <tr>
      <th scope="row" class="text-nowrap"> {'| '.repeat(level)+$types[what].name} </th>
      <td>{name}</td>
      <td class="text-right"> {new Date(attached).toLocaleDateString()} </td >
      <Usage {part_id} />
      <th> <NewPart title='New' cat={$types[what]}/></th>
    </tr>
    <svelte:self hook={what} {subparts} level={level+1}></svelte:self>
  {/each}

{/if}
{:else}
   No subparts maintained!
{/if}