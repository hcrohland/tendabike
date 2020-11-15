<script>
  import {filterValues, parts, attachments, by} from './store'
  import Usage from './Usage.svelte'
 
  export let id;
  
  $: atts = filterValues($attachments, (a) => a.part_id == id).sort(by("attached"))
  
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
          <td>{new Date(att.attached).toLocaleDateString(navigator.language)}</td>
          <td>
            {#if att.detached}
              {new Date(att.detached).toLocaleDateString(navigator.language)}
            {/if}
          </td>
          <Usage part={att} />
        </tr>
      {/each}
    </tbody>
  </table>
</div>