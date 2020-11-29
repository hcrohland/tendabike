<script lang="ts">
import {filterValues, types, state, category} from '../store'
import Usage from '../Usage.svelte'
import type {Type} from '../types'
import SpareType from './SpareType.svelte';

export let params;

// Cannot use category directly since it 
// is unset during destroy and the router gets confused
let cat = $types[params.category]

category.set(cat);

let spareTypes = filterValues<Type>($types, (t) => t.main == cat.id && t.id != cat.id)

</script>

<table class="table table-hover">
  <thead>
    <tr>
      <th scope="col">Part</th>
      <th scope="col">Name</th>
      <Usage header/>
        <th colspan=2>
          Attached to
        </th>
    </tr>
  </thead>
  <tbody>
    {#each spareTypes as type (type.id)}
      <SpareType {type} />
    {/each}
  </tbody>
</table>