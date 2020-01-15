<script>
import {types, filterValues} from './store.js'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let hook;
export let subparts;
export let level = undefined;

let part

let hooks = filterValues($types, (a) => a.hooks.includes(hook)).sort((a,b) => a.order - b.order)
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
    {#each hooks as type}
      {#each subparts.filter((a) => a.what == type.id && a.hook == hook)
        as {what, name, part_id, attached} (part_id)}
        <slot />  
        <tr>
          <th scope="row" class="text-nowrap"> {'| '.repeat(level)+type.name} </th>
          <td>{name}</td>
          <td class="text-right"> {new Date(attached).toLocaleDateString()} </td >
          <Usage {part_id} />
          <th> <NewPart title='New' cat={type}/></th>
        </tr>
        
        <svelte:self hook={type.id} {subparts} level={level+1}></svelte:self>
      {:else}
        <svelte:self hook={type.id} {subparts} level={level+1}>
          <tr>
            <th scope="row" class="text-nowrap"> {'| '.repeat(level)+type.name} </th>
          </tr>
        </svelte:self>
      
      {/each}
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}