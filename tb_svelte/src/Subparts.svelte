<script>
import {types, filterValues, by} from './store'
import SubType from './SubType.svelte'

export let hook;
export let attachees;

function buildList (list, hook, attachees, level, prefix) {
  const typeList = filterValues($types, (a) => a.hooks.includes(hook.id)).sort((a,b) => a.order - b.order);
  typeList.forEach((type) => {
    let attachments = attachees.filter((a) => a.hook == hook.id && a.what == type.id);
    attachments.sort(by("attached"));
    list.push({attachments, prefix, level, type, hook});
    if (attachments.length > 0) {
      buildList(list, type, attachees, level + 1, "");
    } else {
      let prefix = type.name.split(' ').reverse()[1] || '' // The first word iff there were two (hack!)
      buildList(list, type, attachees, level, prefix)
    }
  })
  return list
}

</script>
{#if attachees.length > 0}
    <table class="table table-hover">
    <thead>
      <SubType header/>
    </thead>
    <tbody>
      {#each buildList([], hook, attachees, 0, "") as item (item.hook.id + "." + item.type.id )}
        <SubType {...item } />
      {/each}
    </tbody>
    </table>
{:else}
   No subparts maintained!
{/if}