<script lang="ts">
  import { Modal, ModalHeader, ModalBody, Table } from 'sveltestrap';
  import { fmtDate } from '../store';
  import type { Activity } from '../types';

  export let activities: Activity[];
  export let isOpen = true;
  const toggle = () => {isOpen = false}

</script>

<Modal {isOpen} {toggle} backdrop={false} transitionOptions={{}}>
  <ModalHeader {toggle}> 
    <slot>Activities</slot>
  </ModalHeader>
  <ModalBody>
    <Table>
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
  </ModalBody>
</Modal>