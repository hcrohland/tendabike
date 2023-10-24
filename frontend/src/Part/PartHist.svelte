<script lang="ts">
  import {filterValues, parts, attachments, types, by, attTime} from '../store'
  import Usage from '../Usage.svelte'
 
  export let id;
  
  $: atts = filterValues($attachments, (a) => a.part_id == id).sort(by("attached"))
  
</script>


{#if atts.length > 0}
<div class="table-responsive">
  <table class="table">
    <thead>
      <tr>
        <th scope="col">Attached to</th>
        <th scope="col"> </th>
        <Usage header/>
      </tr>
    </thead>
    <tbody>
      {#each atts as att (att.attached)}
        <tr>
          <td>
          {#if $parts[att.gear]}
            <a href="#/part/{att.gear}" 
                style={$parts[att.gear].disposed_at ? "text-decoration: line-through;" : ""} 
                class="text-reset">
              {$parts[att.gear].name} 
              {types[att.hook].prefix}
            </a>
          {:else}
            N/A
          {/if}
          <td>{attTime(att)}</td>
          <Usage usage={att} />
        </tr>
      {/each}
    </tbody>
  </table>
</div>
{/if}