<script lang="ts">
  import { TableHeadCell } from "flowbite-svelte";
  import Usage from "../Usage/Usage.svelte";
  interface Props {
    children?: import("svelte").Snippet;
  }

  let { children }: Props = $props();
</script>

<TableHeadCell scope="col">
  {@render children?.()}
</TableHeadCell>
<TableHeadCell scope="col" class="text-end">Date</TableHeadCell>
<TableHeadCell class="text-end">Days</TableHeadCell>
<Usage header />
<TableHeadCell></TableHeadCell>
