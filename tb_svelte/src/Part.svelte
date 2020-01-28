<script>
  import {filterValues, types, parts, attachments, category, by} from './store.js'
  import Subparts from './Subparts.svelte'
  import Usage from './Usage.svelte'
 
  export let params;
  
  let part = $parts[params.id]
  category.set($types[$types[part.what].main])

  $: atts = filterValues($attachments, (a) => a.part_id == part.id).sort(by("attached"))
  
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
        <th scope="col">{$types[part.what].name}</th>
        <th scope="col">Brand</th>
        <th scope="col">Model</th>
        <Usage header/>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{part.name}</td>
        <td>{part.vendor}</td>
        <td>{part.model}</td>
        <Usage {part} />
      </tr>
    </tbody>
  </table>
  <table class="table">
  <thead>
    <tr>
      <th scope="col">attached to</th>
      <th scope="col">from </th>
      <th scope="col">until</th>
      <Usage header/>
    </tr>
  </thead>
    <tbody>
      {#each atts as att (att.attached)}
        <tr>
          <td>
          {#if $parts[att.gear]}
            <a href="#/gear/{att.gear}" class="text-reset">
            {$parts[att.gear].name}
            </a>
          {:else}
            N/A
          {/if}
          <td>{new Date(att.attached).toLocaleDateString()}</td>
          <td>
            {#if att.detached}
              {new Date(att.detached).toLocaleDateString()}
            {/if}
          </td>
          <Usage part={att} />
        </tr>
      {/each}
    </tbody>
  </table>
</div>