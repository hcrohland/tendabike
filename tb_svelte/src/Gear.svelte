<script lang="ts" context="module">
  let show_hist_m = false;
</script>
<script lang="ts">
  import {filterValues, types, parts, isAttached, attachments, category} from './store'
  import Subparts from './Subparts.svelte'
  import Usage from './Usage.svelte'
 
  export let params;
  
  let hook, gear;
  $: {
    gear = $parts[params.id]; 
    hook = $types[gear.what];
    category.set(hook)
  }
  $: attachees = filterValues(
    $attachments, 
    (a) => a.gear == gear.id
  )
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
        <th scope="col">{hook.name}</th>
        <th scope="col">Brand</th>
        <th scope="col">Model</th>
        <th scope="col">Purchase</th>
        <Usage header/>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{gear.name}</td>
        <td>{gear.vendor}</td>
        <td>{gear.model}</td>
        <td>{new Date(gear.purchase).toLocaleDateString(navigator.language)}</td>
        <Usage part={gear} />
      </tr>
    </tbody>
  </table>
  <Subparts {hook} {attachees} />
</div>