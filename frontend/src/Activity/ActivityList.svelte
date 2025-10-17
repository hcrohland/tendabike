<script lang="ts">
  import { Modal, Table } from "flowbite-svelte";
  import { fmtDate } from "../lib/store";
  import { Activity } from "../lib/activity";

  export let activities: Activity[];
  export let isOpen = true;
  const toggle = () => {
    isOpen = false;
  };
</script>

<Modal form open={isOpen}>
  {#snippet header()}
    <slot>Activities</slot>
  {/snippet}

  <Table hoverable>
    <tr>
      <th>Start</th>
      <th>Name</th>
    </tr>
    {#each activities as act (act.id)}
      <tr>
        <td>{fmtDate(act.start)}</td>
        <td>{act.name}</td>
      </tr>
    {/each}
  </Table>
</Modal>
