<script lang="ts">
  import { filterValues, by } from "../lib/mapable";
  import { parts, attachments, types } from "../lib/store";
  import Usage from "../Usage.svelte";
  import { link } from "svelte-spa-router";

  export let id: number;

  $: atts = filterValues($attachments, (a) => a.part_id == id).sort(
    by("attached"),
  );
</script>

{#if atts.length > 0}
  <div class="table-responsive">
    <table class="table">
      <thead>
        <tr>
          <th scope="col">Attached to</th>
          <th scope="col"> </th>
          <Usage header />
        </tr>
      </thead>
      <tbody>
        {#each atts as att (att.attached)}
          <tr>
            <td>
              {#if $parts[att.gear]}
                <a
                  href="/part/{att.gear}"
                  use:link
                  style={$parts[att.gear].disposed_at
                    ? "text-decoration: line-through;"
                    : ""}
                  class="text-reset"
                >
                  {$parts[att.gear].name}
                  {types[att.hook].prefix}
                </a>
              {:else}
                N/A
              {/if}
            </td><td>{att.fmtTime()}</td>
            <Usage id={att.usage} ref={att.idx} />
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
