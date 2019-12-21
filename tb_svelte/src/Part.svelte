<script>
  import {parts} from './store.js'
 
  export let params;
  let gear = $parts[params.id]
</script>

<table class="table">
  <thead>
    <tr>
      <th scope="col">Name</th>
      <th scope="col">Brand</th>
      <th scope="col">Model</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>{gear.name}</td>
      <td>{gear.vendor}</td>
      <td>{gear.model}</td>
    </tr>
  </tbody>
</table>