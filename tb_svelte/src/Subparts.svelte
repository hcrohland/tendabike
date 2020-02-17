<script>
import {types, parts, filterValues, by} from './store.js'
import SubType from './SubType.svelte'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let hook;
export let attachees;
export let level = -1;
export let parent = false;

if (parent) level += 1;

const hooks = filterValues($types, (a) => a.hooks.includes(hook.id)).sort((a,b) => a.order - b.order);
  
function findIt(ps, as, type) {
  let atts = as.filter((a) => a.hook == hook.id && a.what == type.id)
  atts.forEach(att => att.part = $parts[att.part_id]); 
  return atts.sort(by("attached"))
}

</script>
{#if attachees.length > 0}
  {#if level === -1}
    <table class="table table-hover">
    <thead>
      <SubType header/>
    </thead>
    <tbody>
      <svelte:self {hook} {attachees} {level} parent></svelte:self>
    </tbody>
    </table>
    
  {:else}
    {#each hooks as type (type.id)}
      <SubType subs={findIt($parts, attachees, type)} {type} {level} bind:parent/>
      <svelte:self hook={type} {attachees} {level} {parent} />
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}