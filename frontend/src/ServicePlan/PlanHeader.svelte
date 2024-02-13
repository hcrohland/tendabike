<script lang="ts">
  import Usage from "../Usage/Usage.svelte";
</script>

<tr>
  <th scope="col">
    <slot />
  </th>
  <th scope="col">Date</th>
  <th class="text-end"> Days</th>
  <Usage header />
  <th />
</tr>
