<script>
  import {myfetch, handleError, types, parts, category} from './store.js'
  import Await from './Await.svelte'
  import Subparts from './Subparts.svelte'
  import Usage from './Usage.svelte'
 
  export let params;
  let gear = $parts[params.id]
  category.set($types[gear.what])

  let promise; 
  promise = myfetch('/attach/to/' + gear.id) 
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
        <th scope="col">{$types[gear.what].name}</th>
        <th scope="col">Brand</th>
        <th scope="col">Model</th>
        <Usage />
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>{gear.name}</td>
        <td>{gear.vendor}</td>
        <td>{gear.model}</td>
        <Usage part_id={gear.id} />
      </tr>
    </tbody>
  </table>
  {#await promise}
    <Await />
  {:then subparts}
    <Subparts hook={gear.what} {subparts} />
  {:catch error}
    handleError(error)
  {/await}
</div>