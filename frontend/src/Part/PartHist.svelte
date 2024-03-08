<script lang="ts">
  import { filterValues, by } from "../lib/mapable";
  import { types } from "../lib/types";
  import Usage from "../Usage/Usage.svelte";
  import PartLink from "./PartLink.svelte";
  import { parts } from "../lib/part";
  import { attachments } from "../lib/attachment";
  import { Table } from "@sveltestrap/sveltestrap";

  export let id: number;

  $: atts = filterValues($attachments, (a) => a.part_id == id).sort(
    by("attached"),
  );
</script>

{#if atts.length > 0}
  <Table responsive hover>
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
              <PartLink part={$parts[att.gear]} />
              {types[att.hook].prefix}
            {:else}
              N/A
            {/if}
          </td><td>{att.fmtTime()}</td>
          <Usage id={att.usage} ref={att.idx} />
        </tr>
      {/each}
    </tbody>
  </Table>
{/if}
