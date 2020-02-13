<script context="module">
  let show_hist_m = false;
</script>
<script>
  import {myfetch, handleError, filterValues, types, parts, isAttached, attachments, category} from './store.js'
  import Await from './Await.svelte'
  import Subparts from './Subparts.svelte'
  import Usage from './Usage.svelte'
 
  export let params;
  let show_hist = show_hist_m;
  $: show_hist_m = show_hist;

  let time = new Date();
  let hook, gear
  $: {
    gear = $parts[params.id]; 
    hook = gear.what
    category.set($types[hook])
  }
  $: attachees = filterValues(
    $attachments, 
    (a) => a.gear == gear.id && (show_hist || isAttached(a, time))
  )
</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>
<span class="badge float-right">
  Show history <input type="checkbox" name="Show history" id="" bind:checked={show_hist}>  
</span>
<div class="scroll-x">
  <table class="table">
    <thead>
      <tr>
        <th scope="col">{$types[hook].name}</th>
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
        <td>{new Date(gear.purchase).toLocaleDateString()}</td>
        <Usage part={gear} />
      </tr>
    </tbody>
  </table>
  <Subparts {hook} {attachees} />
</div>