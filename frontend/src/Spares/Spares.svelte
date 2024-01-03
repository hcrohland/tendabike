<script lang="ts">
import {filterValues, types, state, category} from '../store'
import Usage from '../Usage.svelte'
import SpareType from './SpareType.svelte';
import { Table } from '@sveltestrap/sveltestrap';

export let params;

// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = types[params.category]
let attachee = 0;

category.set(cat);

let spareTypes = filterValues(types, (t) => t.main == cat.id && t.id != cat.id)

function update(show: boolean) {
  show ? attachee++ : attachee--
}
</script>

<div class="table-responsive">
  <Table responsive hover>
  <thead>
    <tr>
      <th scope="col">Part</th>
      <th scope="col">Name</th>
      <Usage header/>
      {#if attachee > 0}
      <th colspan=2>
        Attached to
      </th>
      {/if}
    </tr>
  </thead>
  <tbody>
    {#each spareTypes as type (type.id)}
      <SpareType {type} {attachee} {update}/>
    {/each}
  </tbody>
</Table>
</div>