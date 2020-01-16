<script>
import {types, filterValues} from './store.js'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let hook;
export let subparts;
export let level = undefined;
export let prefix = undefined;

let part

if (prefix) {
  prefix = prefix.split(' ').reverse()[1] // The first word iff there were two (hack!)
} 

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
    {#each hooks as type (type.id)}
      {#if part = subparts.find((a) => a.what == type.id && a.hook == hook)}
        <tr>
          <th scope="row" class="text-nowrap"> 
            {'| '.repeat(level)}
            {#if prefix && type.hooks.length > 1}
              {prefix} 
            {/if}
            {type.name} 
          </th>
          <td>{part.name}</td>
          <td class="text-right"> {new Date(part.attached).toLocaleDateString()} </td >
          <Usage part_id={part.part_id} />
        </tr>
        <svelte:self hook={type.id} {subparts} level={level+1} /> 
      {:else}
        <svelte:self hook={type.id} {subparts} {level} prefix={type.name} />
      {/if}
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}