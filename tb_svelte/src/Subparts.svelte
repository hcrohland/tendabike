<script>
import {types, parts, filterValues, by} from './store.js'
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

function findIt(ps, type) {
  let atts = attachees.filter((a) => a.hook == hook && a.what == type.id)
  atts.forEach(att => att.part = $parts[att.part_id]); 
  return atts.sort(by("attached"))
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
        <Usage header/>
        <th></th>
      </tr>
    </thead>
    <tbody>
      <svelte:self {hook} {attachees} level={0}></svelte:self>
    </tbody>
    </table>
    
  {:else}
    {#each hooks as type (type.id)}
      {#each findIt ($parts, type) as att,i (att.attached)}
        <tr>
          <th scope="row" class="text-nowrap"> 
            {'| '.repeat(level)}
            {#if prefix && type.hooks.length > 1}
              {prefix} 
            {/if}
            {#if i == 0}
              {type.name} 
            {:else}
              |
            {/if}
          </th>
          <td>
          {#if att.part}
          <a href="#/part/{att.part_id}" disabled={att.part===null} class="text-reset">
            {att.name}
          </a>
          {:else}
            {att.name}
          {/if}
          </td>
          <td class="text-right"> {new Date(att.attached).toLocaleDateString()} </td >
          <Usage part={att} />
        </tr>
        <svelte:self hook={type.id} {attachees} level={level+1} /> 
      {:else}
        <svelte:self hook={type.id} {attachees} {level} prefix={type.name} />
      {/each}
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}