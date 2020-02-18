<script>
import {types, parts, filterValues, by} from './store.js'
import SubType from './SubType.svelte'
import Usage from './Usage.svelte'
import NewPart from './NewPart.svelte'

export let type;
export let attachees;
export let level = -1;
export let prefix = undefined;

if (prefix == "") level += 1;

const typeList = filterValues($types, (a) => a.hooks.includes(type.id)).sort((a,b) => a.order - b.order);

$: hooks = typeList.map(h => {
    h.subs = attachees.filter((a) => a.hook == type.id && a.what == h.id)
    h.subs.forEach(att => att.part = $parts[att.part_id]); 
    h.subs.sort(by("attached"))
    if (h.subs.length == 0) {
      h.prefix = h["name"].split(' ').reverse()[1] || ""// The first word iff there were two (hack!)
    } else {
      h.prefix = ""
    }
    return h
  })


</script>
{#if attachees.length > 0}
  {#if prefix === undefined}
    <table class="table table-hover">
    <thead>
      <SubType header/>
    </thead>
    <tbody>
      <svelte:self {type} {attachees} {level} prefix={""} />
    </tbody>
    </table>
    
  {:else}
    {#each hooks as type (type.id)}
      <SubType {type} {level} {prefix}/>
      <svelte:self {type} {attachees} {level} prefix={type.prefix} />
    {/each}
  {/if}
{:else}
   No subparts maintained!
{/if}