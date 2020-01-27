<script>
import {types, parts, filterValues} from './store.js'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let hook;
export let attachees;
export let level = undefined;
export let prefix = undefined;

let part;
const hooks = filterValues($types, (a) => a.hooks.includes(hook)).sort((a,b) => a.order - b.order);

if (prefix) {
  prefix = prefix.split(' ').reverse()[1] // The first word iff there were two (hack!)
} 

function findIt(atts, ps, type) {
  let att = atts.find((a) =>  ps[a.part_id].what == type.id && a.hook == hook)
  if (att) {
    let part = $parts[att.part_id];
    part.attached = att.attached;
    return part
  }
  return undefined
}

</script>
{#if attachees.length > 0}
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
      <svelte:self {hook} {attachees} level={0}></svelte:self>
    </tbody>
    </table>
    
  {:else}
    {#each hooks as type (type.id)}
      {#if part = findIt (attachees, $parts, type)}
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
          <Usage part_id={part.id} />
        </tr>
        <svelte:self hook={type.id} {attachees} level={level+1} /> 
      {:else}
        <svelte:self hook={type.id} {attachees} {level} prefix={type.name} />
      {/if}
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}