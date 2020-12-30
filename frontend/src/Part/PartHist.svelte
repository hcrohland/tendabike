<script lang="ts">
  import {filterValues, parts, attachments, types, by, fmtDate, maxDate} from '../store'
  import Usage from '../Usage.svelte'
 
  export let id;
  
  $: atts = filterValues($attachments, (a) => a.part_id == id).sort(by("attached"))
  
</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>
{#if atts.length > 0}
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
            <a href="#/gear/{att.gear}" 
                style={$parts[att.gear].disposed_at ? "text-decoration: line-through;" : ""} 
                class="text-reset">
              {$parts[att.gear].name} 
              {types[att.hook].prefix}
            </a>
          {:else}
            N/A
          {/if}
          <td>{fmtDate(att.attached)}</td>
          <td>
            {#if att.detached < maxDate}
              {fmtDate(att.detached)}
            {/if}
          </td>
          <Usage usage={att} />
        </tr>
      {/each}
    </tbody>
  </table>
</div>
{/if}