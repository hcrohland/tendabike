<script lang="ts">
  import { TableHeadCell } from "flowbite-svelte";
  import Usage from "../Usage/Usage.svelte";
</script>

<TableHeadCell scope="col">
  <slot />
</TableHeadCell>
<TableHeadCell scope="col">Date</TableHeadCell>
<TableHeadCell scope="col" class="text-end">Days</TableHeadCell>
<Usage header />
<TableHeadCell></TableHeadCell>
