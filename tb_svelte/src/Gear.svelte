<script>
  import {myfetch, handleError, filterValues, types, parts, isAttached, attachments, category} from './store.js'
  import Await from './Await.svelte'
  import Subparts from './Subparts.svelte'
  import Usage from './Usage.svelte'
 
  export let params;
  
  let show_all = false;
  let time = new Date();
  let hook = $parts[params.id].what
  category.set($types[hook])

  $: gear = $parts[params.id]; 
  $: attachees = filterValues(
    $attachments, 
    (a) => a.gear == gear.id && (show_all || isAttached(a, time))
  )
  

</script>

<style>
.scroll-x {
  width: 100%;
  overflow-x: scroll;
}
</style>
<span class="badge float-right">
  Show history <input type="checkbox" name="Show all" id="" bind:checked={show_all}>  
</span>
<div class="scroll-x">
  <table class="table">
    <thead>
      <tr>
        <th scope="col">{$types[hook].name}</th>
        <th scope="col">Brand</th>
        <th scope="col">Model</th>
        <Usage header/>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{gear.name}</td>
        <td>{gear.vendor}</td>
        <td>{gear.model}</td>
        <Usage part={gear} />
      </tr>
    </tbody>
  </table>
  <Subparts {hook} {attachees} />
</div>